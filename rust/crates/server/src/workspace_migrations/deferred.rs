#![allow(dead_code)]

use serde_json::Value;
use sqlx::pool::PoolConnection;
use sqlx::PgConnection;
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use super::column_defs::composite_types;
use super::ddl::foreign_key;
use super::ddl::foreign_key::WorkspaceSchemaForeignKeyDefinition;
use super::ddl::index;
use super::ddl::index::WorkspaceSchemaIndexDefinition;
use super::runner::object_table_name;
use super::runner::RunnerError;
use super::runner::WorkspaceRuntime;
use crate::metadata::names::compute_composite_column_name;
use crate::metadata::names::compute_relation_join_column_name;
use crate::state::AppState;

pub const BUILD_INDEX: &str = "build_index";
pub const VALIDATE_FOREIGN_KEY: &str = "validate_foreignKey";
pub const REBUILD_OBJECT: &str = "rebuild_object";

pub const SCHEMA_AFFECTING_DEFERRED_OPERATIONS: [&str; 3] =
    [BUILD_INDEX, VALIDATE_FOREIGN_KEY, REBUILD_OBJECT];

pub const MAX_ATTEMPTS: i32 = 3;
pub const DEFAULT_RUN_BY_VERSION: &str = "nudge-0.1.0";
pub const DEFERRED_STATEMENT_TIMEOUT_MS: i64 = 60 * 60 * 1000;
pub const DEFERRED_STALE_THRESHOLD_MS: i64 = DEFERRED_STATEMENT_TIMEOUT_MS + 15 * 60 * 1000;
pub const INITIAL_BACKOFF_MS: u64 = 30_000;
pub const STALE_IN_PROGRESS_ERROR: &str = "Worker stopped while the operation was in progress";

pub fn next_attempt_backoff_ms(attempts: i32) -> u64 {
    if attempts <= 0 {
        return 0;
    }

    let exponent = (attempts - 1).min(20) as u32;

    INITIAL_BACKOFF_MS.checked_shl(exponent).unwrap_or(u64::MAX)
}

#[derive(Debug, Clone)]
pub struct DeferredOperationInput {
    pub name: String,
    pub payload: Value,
    pub application_universal_identifier: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeferredWorkspaceOperation {
    pub id: Uuid,
    #[sqlx(rename = "workspaceId")]
    pub workspace_id: Uuid,
    #[sqlx(rename = "applicationUniversalIdentifier")]
    pub application_universal_identifier: String,
    pub name: String,
    pub payload: Value,
    pub position: i32,
    #[sqlx(rename = "runByVersion")]
    pub run_by_version: Option<String>,
    pub status: String,
    pub attempts: i32,
    #[sqlx(rename = "lastError")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferredStatus {
    Pending,
    InProgress,
    Failed,
}

impl DeferredStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            DeferredStatus::Pending => "PENDING",
            DeferredStatus::InProgress => "IN_PROGRESS",
            DeferredStatus::Failed => "FAILED",
        }
    }
}

pub fn status_after_failure(attempts: i32) -> DeferredStatus {
    if attempts >= MAX_ATTEMPTS {
        DeferredStatus::Failed
    } else {
        DeferredStatus::Pending
    }
}

pub fn is_schema_affecting_name(name: &str) -> bool {
    SCHEMA_AFFECTING_DEFERRED_OPERATIONS.contains(&name)
}

pub async fn has_in_flight_schema_affecting(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<bool, RunnerError> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM core.\"deferredWorkspaceOperation\" \
         WHERE \"workspaceId\" = $1 AND status IN ('PENDING', 'IN_PROGRESS') \
         AND name = ANY($2))",
    )
    .bind(workspace_id)
    .bind(&SCHEMA_AFFECTING_DEFERRED_OPERATIONS[..])
    .fetch_one(&state.core_pool)
    .await?;

    Ok(row.0)
}

pub async fn persist_many(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    inputs: &[DeferredOperationInput],
) -> Result<(), RunnerError> {
    let connection: &mut PgConnection = transaction;

    let start_position: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(position), -1)::bigint + 1 \
         FROM core.\"deferredWorkspaceOperation\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_one(&mut *connection)
    .await?;

    for (offset, input) in inputs.iter().enumerate() {
        sqlx::query(
            "INSERT INTO core.\"deferredWorkspaceOperation\" \
             (id, \"workspaceId\", \"applicationUniversalIdentifier\", name, payload, \
              \"position\", \"runByVersion\", status, attempts) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'PENDING', 0)",
        )
        .bind(Uuid::new_v4())
        .bind(workspace_id)
        .bind(&input.application_universal_identifier)
        .bind(&input.name)
        .bind(&input.payload)
        .bind((start_position.0 + offset as i64) as i32)
        .bind(DEFAULT_RUN_BY_VERSION)
        .execute(&mut *connection)
        .await?;
    }

    Ok(())
}

pub async fn claim_pending(
    state: &AppState,
    workspace_id: Uuid,
    limit: i64,
) -> Result<Vec<DeferredWorkspaceOperation>, RunnerError> {
    claim_rows(state, workspace_id, limit, false).await
}

pub async fn claim_pending_backoff(
    state: &AppState,
    workspace_id: Uuid,
    limit: i64,
) -> Result<Vec<DeferredWorkspaceOperation>, RunnerError> {
    claim_rows(state, workspace_id, limit, true).await
}

async fn claim_rows(
    state: &AppState,
    workspace_id: Uuid,
    limit: i64,
    respect_backoff: bool,
) -> Result<Vec<DeferredWorkspaceOperation>, RunnerError> {
    let backoff_filter = if respect_backoff {
        "AND CASE WHEN attempts <= 0 THEN 0 \
         ELSE ($3::bigint * power(2, attempts - 1))::bigint END \
         <= (extract(epoch from (now() - \"updatedAt\")) * 1000)::bigint"
    } else {
        ""
    };

    let sql = format!(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'IN_PROGRESS', \
         \"startedAt\" = now(), attempts = attempts + 1, \"updatedAt\" = now() \
         WHERE id IN ( \
             SELECT id FROM core.\"deferredWorkspaceOperation\" \
             WHERE \"workspaceId\" = $1 AND status = 'PENDING' {backoff_filter} \
             ORDER BY \"createdAt\" ASC, position ASC LIMIT $2 \
             FOR UPDATE SKIP LOCKED \
         ) RETURNING *"
    );

    let mut query = sqlx::query_as(&sql).bind(workspace_id).bind(limit);

    if respect_backoff {
        query = query.bind(INITIAL_BACKOFF_MS as i64);
    }

    let rows: Vec<DeferredWorkspaceOperation> = query.fetch_all(&state.core_pool).await?;

    Ok(rows)
}

pub async fn run_pending_inline(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<usize, RunnerError> {
    let mut executed = 0;

    'outer: loop {
        let claimed = claim_pending(state, workspace_id, 10).await?;

        if claimed.is_empty() {
            break;
        }

        for operation in claimed {
            if let Err(error) = execute_and_record(state, workspace_id, &operation).await {
                tracing::warn!(
                    operation_id = %operation.id,
                    %error,
                    "inline deferred operation failed; the queue worker will retry it"
                );

                break 'outer;
            }

            executed += 1;
        }
    }

    Ok(executed)
}

fn truncate_error_message(message: &str) -> String {
    const MAX_ERROR_BYTES: usize = 3900;

    if message.len() <= MAX_ERROR_BYTES {
        return message.to_string();
    }

    let mut end = MAX_ERROR_BYTES;

    while !message.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}…", &message[..end])
}

pub async fn execute_and_record(
    state: &AppState,
    workspace_id: Uuid,
    operation: &DeferredWorkspaceOperation,
) -> Result<(), RunnerError> {
    match execute(state, workspace_id, operation).await {
        Ok(()) => {
            delete_claimed(state, operation.id, operation.attempts).await?;

            Ok(())
        }
        Err(error) => {
            let message = truncate_error_message(&error.to_string());

            if operation.attempts >= MAX_ATTEMPTS {
                mark_failed(state, operation.id, operation.attempts, &message).await?;
            } else {
                mark_pending_again(state, operation.id, operation.attempts, &message).await?;
            }

            Err(error)
        }
    }
}

async fn delete_claimed(
    state: &AppState,
    operation_id: Uuid,
    attempts: i32,
) -> Result<(), RunnerError> {
    sqlx::query(
        "DELETE FROM core.\"deferredWorkspaceOperation\" \
         WHERE id = $1 AND status = 'IN_PROGRESS' AND attempts = $2",
    )
    .bind(operation_id)
    .bind(attempts)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

async fn mark_failed(
    state: &AppState,
    operation_id: Uuid,
    attempts: i32,
    last_error: &str,
) -> Result<(), RunnerError> {
    sqlx::query(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'FAILED', \
         \"lastError\" = $2, \"updatedAt\" = now() \
         WHERE id = $1 AND status = 'IN_PROGRESS' AND attempts = $3",
    )
    .bind(operation_id)
    .bind(last_error)
    .bind(attempts)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

async fn mark_pending_again(
    state: &AppState,
    operation_id: Uuid,
    attempts: i32,
    last_error: &str,
) -> Result<(), RunnerError> {
    sqlx::query(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'PENDING', \
         \"lastError\" = $2, \"updatedAt\" = now() \
         WHERE id = $1 AND status = 'IN_PROGRESS' AND attempts = $3",
    )
    .bind(operation_id)
    .bind(last_error)
    .bind(attempts)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

pub async fn reset_failed(state: &AppState, workspace_id: Uuid) -> Result<u64, RunnerError> {
    let affected = sqlx::query(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'PENDING', \
         attempts = 0, \"lastError\" = NULL, \"updatedAt\" = now() \
         WHERE \"workspaceId\" = $1 AND status = 'FAILED'",
    )
    .bind(workspace_id)
    .execute(&state.core_pool)
    .await?
    .rows_affected();

    Ok(affected)
}

async fn resolve_deferred_runtime(
    core_pool: &sqlx::PgPool,
    workspace_id: Uuid,
    application_universal_identifier: &str,
) -> Result<WorkspaceRuntime, RunnerError> {
    let workspace: Option<(Option<String>, Option<Uuid>)> = sqlx::query_as(
        "SELECT \"databaseSchema\", \"workspaceCustomApplicationId\" \
         FROM core.\"workspace\" WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(workspace_id)
    .fetch_optional(core_pool)
    .await?;

    let (schema_name, custom_application_id) =
        workspace.ok_or_else(|| RunnerError::workspace_not_found(workspace_id))?;

    let schema_name = schema_name.ok_or_else(|| {
        RunnerError::execution_failed(format!("Workspace {workspace_id} has no database schema"))
    })?;

    let application: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, \"universalIdentifier\"::text FROM core.\"application\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\"::text = $2",
    )
    .bind(workspace_id)
    .bind(application_universal_identifier)
    .fetch_optional(core_pool)
    .await?;

    let (application_id, application_universal_identifier) = match application {
        Some(row) => row,
        None => {
            let custom_id = custom_application_id.ok_or_else(|| {
                RunnerError::application_not_found(application_universal_identifier)
            })?;

            let fallback: Option<(Uuid, String)> = sqlx::query_as(
                "SELECT id, \"universalIdentifier\"::text FROM core.\"application\" WHERE id = $1",
            )
            .bind(custom_id)
            .fetch_optional(core_pool)
            .await?;

            fallback.ok_or_else(|| {
                RunnerError::application_not_found(application_universal_identifier)
            })?
        }
    };

    Ok(WorkspaceRuntime {
        workspace_id,
        schema_name,
        application_id,
        application_universal_identifier,
    })
}

pub async fn execute(
    state: &AppState,
    workspace_id: Uuid,
    operation: &DeferredWorkspaceOperation,
) -> Result<(), RunnerError> {
    let runtime = resolve_deferred_runtime(
        &state.core_pool,
        workspace_id,
        &operation.application_universal_identifier,
    )
    .await?;

    let mut connection = state.core_pool.acquire().await?;

    sqlx::query(&format!(
        "SET statement_timeout = '{DEFERRED_STATEMENT_TIMEOUT_MS}ms'"
    ))
    .execute(&mut *connection)
    .await?;

    let result = match operation.name.as_str() {
        BUILD_INDEX => execute_build_index(&mut connection, &runtime, operation).await,
        VALIDATE_FOREIGN_KEY => {
            execute_validate_foreign_key(&mut connection, &runtime, &operation.payload).await
        }
        REBUILD_OBJECT => {
            execute_rebuild_object(&mut connection, &runtime, &operation.payload).await
        }
        other => Err(RunnerError::execution_failed(format!(
            "unsupported deferred operation {other}"
        ))),
    };

    let _ = sqlx::query("RESET statement_timeout")
        .execute(&mut *connection)
        .await;

    result
}

pub async fn reset_stale_in_progress(state: &AppState) -> Result<u64, RunnerError> {
    let failed = sqlx::query(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'FAILED', \
         \"lastError\" = $1, \"updatedAt\" = now() \
         WHERE status = 'IN_PROGRESS' \
         AND \"startedAt\" < now() - $2 * interval '1 millisecond' \
         AND attempts >= $3",
    )
    .bind(STALE_IN_PROGRESS_ERROR)
    .bind(DEFERRED_STALE_THRESHOLD_MS)
    .bind(MAX_ATTEMPTS)
    .execute(&state.core_pool)
    .await?
    .rows_affected();

    let pending = sqlx::query(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'PENDING', \
         \"lastError\" = $1, \"updatedAt\" = now() \
         WHERE status = 'IN_PROGRESS' \
         AND \"startedAt\" < now() - $2 * interval '1 millisecond' \
         AND attempts < $3",
    )
    .bind(STALE_IN_PROGRESS_ERROR)
    .bind(DEFERRED_STALE_THRESHOLD_MS)
    .bind(MAX_ATTEMPTS)
    .execute(&state.core_pool)
    .await?
    .rows_affected();

    Ok(failed + pending)
}

async fn execute_build_index(
    connection: &mut PoolConnection<Postgres>,
    runtime: &WorkspaceRuntime,
    operation: &DeferredWorkspaceOperation,
) -> Result<(), RunnerError> {
    let payload = &operation.payload;

    let index_metadata_id = payload
        .get("indexMetadataId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunnerError::execution_failed("build_index payload needs indexMetadataId")
        })?;

    let index_metadata_id = Uuid::parse_str(index_metadata_id)
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let row: Option<(String, Uuid, bool, Option<String>, String)> = sqlx::query_as(
        "SELECT name, \"objectMetadataId\", \"isUnique\", \"indexWhereClause\", \"indexType\"::text \
         FROM core.\"indexMetadata\" WHERE id = $1",
    )
    .bind(index_metadata_id)
    .fetch_optional(&mut **connection)
    .await?;

    let Some((name, object_id, is_unique, where_clause, index_type)) = row else {
        return Ok(());
    };

    let table_name = object_table_name(connection, object_id).await?;

    let index_fields: Vec<(Option<String>, String, String)> = sqlx::query_as(
        "SELECT ifm.\"subFieldName\", f.name, f.type \
         FROM core.\"indexFieldMetadata\" ifm \
         JOIN core.\"fieldMetadata\" f ON f.id = ifm.\"fieldMetadataId\" \
         WHERE ifm.\"indexMetadataId\" = $1 ORDER BY ifm.\"order\" ASC",
    )
    .bind(index_metadata_id)
    .fetch_all(&mut **connection)
    .await?;

    let column_names = index_column_names_for_fields(&index_fields);

    let definition = WorkspaceSchemaIndexDefinition {
        name,
        columns: column_names,
        is_unique,
        index_type: Some(index_type),
        where_clause,
    };

    if operation.attempts > 1 {
        index::drop_index(connection, &runtime.schema_name, &definition.name, true)
            .await
            .map_err(|error| RunnerError::execution_failed(error.to_string()))?;
    }

    index::create_index_concurrently(connection, &runtime.schema_name, &table_name, &definition)
        .await
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    Ok(())
}

fn index_column_names_for_fields(index_fields: &[(Option<String>, String, String)]) -> Vec<String> {
    let mut columns: Vec<String> = Vec::new();

    for (sub_field_name, field_name, field_type) in index_fields {
        let Some(parsed) = crate::metadata::flat::FieldMetadataType::parse(field_type) else {
            continue;
        };

        let ddl_type = super::runner::ddl_field_type(parsed);

        if matches!(
            parsed,
            crate::metadata::flat::FieldMetadataType::Relation
                | crate::metadata::flat::FieldMetadataType::MorphRelation
        ) {
            columns.push(compute_relation_join_column_name(field_name));
            continue;
        }

        if composite_types::is_composite_field_metadata_type(ddl_type) {
            let properties = composite_types::composite_properties(ddl_type);

            if let Some(sub_field_name) = sub_field_name {
                if properties
                    .iter()
                    .any(|property| property.name == *sub_field_name)
                {
                    columns.push(compute_composite_column_name(field_name, sub_field_name));
                }

                continue;
            }

            for property in properties {
                if property.is_included_in_unique_constraint {
                    columns.push(compute_composite_column_name(field_name, property.name));
                }
            }

            continue;
        }

        columns.push(field_name.clone());
    }

    columns
}

async fn execute_validate_foreign_key(
    connection: &mut PoolConnection<Postgres>,
    runtime: &WorkspaceRuntime,
    payload: &Value,
) -> Result<(), RunnerError> {
    let field_metadata_id = payload
        .get("fieldMetadataId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunnerError::execution_failed("validate_foreignKey payload needs fieldMetadataId")
        })?;

    let field_id = Uuid::parse_str(field_metadata_id)
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let row: Option<(Uuid, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT \"objectMetadataId\", name, \"relationTargetObjectMetadataId\" \
         FROM core.\"fieldMetadata\" WHERE id = $1",
    )
    .bind(field_id)
    .fetch_optional(&mut **connection)
    .await?;

    let Some((object_id, field_name, target_object_id)) = row else {
        return Ok(());
    };

    let Some(target_object_id) = target_object_id else {
        return Ok(());
    };

    let table_name = object_table_name(connection, object_id).await?;
    let target_table = object_table_name(connection, target_object_id).await?;

    let foreign_key = WorkspaceSchemaForeignKeyDefinition {
        table_name,
        column_name: format!("{field_name}Id"),
        referenced_table_name: target_table,
        referenced_column_name: "id".to_string(),
        on_delete: Some("CASCADE".to_string()),
        on_update: None,
    };

    foreign_key::validate_foreign_key(connection, &runtime.schema_name, &foreign_key)
        .await
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    Ok(())
}

async fn execute_rebuild_object(
    connection: &mut PoolConnection<Postgres>,
    runtime: &WorkspaceRuntime,
    payload: &Value,
) -> Result<(), RunnerError> {
    let object_metadata_id = payload
        .get("objectMetadataId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunnerError::execution_failed("rebuild_object payload needs objectMetadataId")
        })?;

    let object_id = Uuid::parse_str(object_metadata_id)
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let table_name = object_table_name(connection, object_id).await?;

    let rebuilt_table_name = format!("{table_name}__rebuild");

    let table_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_schema = $1 AND table_name = $2)",
    )
    .bind(&runtime.schema_name)
    .bind(&table_name)
    .fetch_one(&mut **connection)
    .await?;

    if !table_exists.0 {
        return Ok(());
    }

    let dropped_column_count: (i64,) = sqlx::query_as(
        "SELECT COALESCE(COUNT(*) FILTER (WHERE a.attisdropped), 0)::bigint \
         FROM pg_class c \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         LEFT JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum > 0 \
         WHERE n.nspname = $1 AND c.relname = $2 AND c.relkind = 'r'",
    )
    .bind(&runtime.schema_name)
    .bind(&table_name)
    .fetch_one(&mut **connection)
    .await?;

    if dropped_column_count.0 == 0 {
        return Ok(());
    }

    sqlx::query(&format!(
        "ALTER TABLE {} RENAME TO {}",
        quoted(&runtime.schema_name, &table_name)?,
        quoted_identifier(&rebuilt_table_name)?
    ))
    .execute(&mut **connection)
    .await?;

    let definitions =
        regenerated_column_definitions(connection, &runtime.schema_name, object_id, &table_name)
            .await?;

    super::ddl::table::create_table(connection, &runtime.schema_name, &table_name, &definitions)
        .await?;

    let shared_columns: Vec<(String,)> = sqlx::query_as(
        "SELECT c.column_name FROM information_schema.columns c \
         JOIN information_schema.columns r \
           ON r.table_schema = c.table_schema \
          AND r.column_name = c.column_name \
          AND r.table_name = $3 \
         WHERE c.table_schema = $1 AND c.table_name = $2 \
           AND c.is_generated = 'NEVER' AND r.is_generated = 'NEVER'",
    )
    .bind(&runtime.schema_name)
    .bind(&table_name)
    .bind(&rebuilt_table_name)
    .fetch_all(&mut **connection)
    .await?;

    let column_list = shared_columns
        .iter()
        .map(|(name,)| quoted_identifier(name))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");

    if !column_list.is_empty() {
        sqlx::query(&format!(
            "INSERT INTO {} ({}) SELECT {} FROM {}",
            quoted(&runtime.schema_name, &table_name)?,
            column_list,
            column_list,
            quoted(&runtime.schema_name, &rebuilt_table_name)?,
        ))
        .execute(&mut **connection)
        .await?;
    }

    super::ddl::table::drop_table(connection, &runtime.schema_name, &rebuilt_table_name, true)
        .await?;

    Ok(())
}

fn quoted(schema_name: &str, table_name: &str) -> Result<String, RunnerError> {
    Ok(format!(
        "{}.{}",
        quoted_identifier(schema_name)?,
        quoted_identifier(table_name)?
    ))
}

fn quoted_identifier(raw: &str) -> Result<String, RunnerError> {
    super::ddl::escape::escape_identifier(raw)
        .map_err(|error| RunnerError::execution_failed(error.to_string()))
}

async fn regenerated_column_definitions(
    connection: &mut PgConnection,
    schema_name: &str,
    object_id: Uuid,
    table_name: &str,
) -> Result<Vec<super::column_defs::WorkspaceSchemaColumnDefinition>, RunnerError> {
    let fields: Vec<(String, String, Option<bool>, Option<Value>)> = sqlx::query_as(
        "SELECT name, type, \"isNullable\", \"defaultValue\" \
         FROM core.\"fieldMetadata\" \
         WHERE \"objectMetadataId\" = $1 AND \"isActive\" ORDER BY \"createdAt\" ASC",
    )
    .bind(object_id)
    .fetch_all(&mut *connection)
    .await?;

    let mut definitions = super::runner::base_column_definitions_for_rebuild();

    for (name, field_type, is_nullable, default_value) in fields {
        let Some(field_type) = crate::metadata::flat::FieldMetadataType::parse(&field_type) else {
            continue;
        };

        let input = super::column_defs::FlatFieldColumnInput {
            name,
            field_type: super::runner::ddl_field_type(field_type),
            is_nullable,
            default_value,
            universal_identifier: String::new(),
            morph_id: None,
            relation_type: None,
        };

        if let Ok(generated) =
            super::column_defs::generate_column_definitions(&input, schema_name, table_name, None)
        {
            definitions.extend(generated);
        }
    }

    Ok(definitions)
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::metadata::actions::MetadataName;
    use crate::metadata::actions::UniversalAction;
    use crate::metadata::actions::UniversalCreateAction;
    use crate::metadata::actions::UniversalFlatEntity;
    use crate::metadata::actions::WorkspaceMigration;
    use crate::metadata::flat::UniversalFlatObjectMetadata;
    use crate::workspace_migrations::runner::run_workspace_migration;
    use crate::workspace_migrations::runner::RunnerInput;
    use nudge_core::Config;
    use nudge_core::NodeEnv;
    use std::time::Duration;

    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn test_config(pg_database_url: String) -> Config {
        Config {
            node_env: NodeEnv::Test,
            node_port: 3999,
            server_url: "http://localhost:3999".to_string(),
            pg_database_url,
            pg_pool_max_connections: 4,
            redis_url: "redis://127.0.0.1:6399".to_string(),
            redis_cache_prefix: String::new(),
            app_secret: "integration-test-secret".to_string(),
            encryption_key: None,
            fallback_encryption_key: None,
            access_token_expires_in_secs: 1800,
            refresh_token_expires_in_secs: 3600,
            login_token_expires_in_secs: 900,
            workspace_agnostic_token_expires_in_secs: 1800,
            refresh_token_reuse_grace_period_secs: 60,
            frontend_url: "http://localhost:3999".to_string(),
            auth_cookie_same_site: "lax".to_string(),
            auth_cookie_allowed_origins: String::new(),
            session_absolute_lifetime_secs: 3600,
            session_idle_timeout_secs: 3600,
            is_cloudflare_geolocation_enabled: false,
            workspace_schema_ddl_locked: false,
            search_ilike_fallback_timeout_ms: 2000,
            allow_requests_to_twenty_icons: true,
            file_token_expires_in_secs: 86400,
            storage_local_path: ".local-storage".into(),
        }
    }

    async fn test_state() -> Option<AppState> {
        let url = std::env::var("NUDGE_TEST_DATABASE_URL").ok()?;
        crate::workspace_migrations::ddl::test_support::migrate_core(&url)
            .await
            .expect("running core migrations");
        AppState::connect(&test_config(url)).await.ok()
    }

    struct TestWorkspace {
        workspace_id: Uuid,
        application_universal_identifier: String,
        schema_name: String,
    }

    async fn create_test_workspace(state: &AppState, table_name: &str) -> TestWorkspace {
        let workspace_id = Uuid::new_v4();
        let application_id = Uuid::new_v4();
        let application_universal_identifier = Uuid::new_v4().to_string();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let subdomain = format!("deferred-{}", &Uuid::new_v4().simple().to_string()[..10]);

        let mut transaction = state.core_pool.begin().await.expect("tx");

        sqlx::query(
            "INSERT INTO core.\"workspace\" \
             (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
              \"workspaceCustomApplicationId\", \"isPasswordAuthEnabled\", \"databaseSchema\", \
              \"defaultRoleId\") \
             VALUES ($1, 'Deferred Test', $2, 'CREATED', $3, $4, true, $5, $6)",
        )
        .bind(workspace_id)
        .bind(Uuid::new_v4())
        .bind(&subdomain)
        .bind(application_id)
        .bind(&schema_name)
        .bind(Uuid::new_v4())
        .execute(&mut *transaction)
        .await
        .expect("inserting the test workspace");

        sqlx::query(
            "INSERT INTO core.\"application\" \
             (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", \"workspaceId\") \
             VALUES ($1, $2, 'Deferred Test Application', 'LOCAL', 'INSTALLED', '', $3)",
        )
        .bind(application_id)
        .bind(
            Uuid::parse_str(&application_universal_identifier)
                .expect("application universal identifier is a uuid"),
        )
        .bind(workspace_id)
        .execute(&mut *transaction)
        .await
        .expect("inserting the test application");

        transaction.commit().await.expect("committing the fixture");

        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name))
            .execute(&state.core_pool)
            .await
            .expect("creating the workspace schema");

        sqlx::query(&format!(
            "CREATE TABLE {}.\"{}\" (id uuid PRIMARY KEY DEFAULT public.uuid_generate_v4(), \
             \"createdAt\" timestamptz NOT NULL DEFAULT now(), \
             \"updatedAt\" timestamptz NOT NULL DEFAULT now(), \
             \"deletedAt\" timestamptz, \"codeText\" text)",
            quoted_identifier(&schema_name).unwrap(),
            table_name
        ))
        .execute(&state.core_pool)
        .await
        .expect("creating the workspace table");

        TestWorkspace {
            workspace_id,
            application_universal_identifier,
            schema_name,
        }
    }

    async fn insert_object_metadata(
        state: &AppState,
        workspace: &TestWorkspace,
        table_name: &str,
    ) -> Uuid {
        let application_id: (Uuid,) =
            sqlx::query_as("SELECT id FROM core.\"application\" WHERE \"workspaceId\" = $1")
                .bind(workspace.workspace_id)
                .fetch_one(&state.core_pool)
                .await
                .expect("reading the application");

        let object_id = Uuid::new_v4();
        let singular = table_name.trim_start_matches('_').to_string();

        sqlx::query(
            "INSERT INTO core.\"objectMetadata\" \
             (id, \"workspaceId\", \"applicationId\", \"nameSingular\", \"namePlural\", \
              \"labelSingular\", \"labelPlural\", \"targetTableName\", \"universalIdentifier\") \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(object_id)
        .bind(workspace.workspace_id)
        .bind(application_id.0)
        .bind(&singular)
        .bind(format!("{singular}s"))
        .bind(&singular)
        .bind(format!("{singular}s"))
        .bind(table_name)
        .bind(Uuid::new_v4())
        .execute(&state.core_pool)
        .await
        .expect("inserting the object metadata");

        object_id
    }

    async fn insert_text_field(
        state: &AppState,
        workspace: &TestWorkspace,
        object_id: Uuid,
        name: &str,
    ) -> Uuid {
        let application_id: (Uuid,) =
            sqlx::query_as("SELECT id FROM core.\"application\" WHERE \"workspaceId\" = $1")
                .bind(workspace.workspace_id)
                .fetch_one(&state.core_pool)
                .await
                .expect("reading the application");

        let field_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO core.\"fieldMetadata\" \
             (id, \"workspaceId\", \"applicationId\", \"objectMetadataId\", name, label, \
              type, \"isActive\", \"universalIdentifier\") \
             VALUES ($1, $2, $3, $4, $5, $6, 'TEXT', true, $7)",
        )
        .bind(field_id)
        .bind(workspace.workspace_id)
        .bind(application_id.0)
        .bind(object_id)
        .bind(name)
        .bind(name)
        .bind(Uuid::new_v4())
        .execute(&state.core_pool)
        .await
        .expect("inserting the field metadata");

        field_id
    }

    async fn insert_deferrable_index(
        state: &AppState,
        workspace: &TestWorkspace,
        object_id: Uuid,
        field_id: Uuid,
        index_name: &str,
    ) -> Uuid {
        let application_id: (Uuid,) =
            sqlx::query_as("SELECT id FROM core.\"application\" WHERE \"workspaceId\" = $1")
                .bind(workspace.workspace_id)
                .fetch_one(&state.core_pool)
                .await
                .expect("reading the application");

        let index_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO core.\"indexMetadata\" \
             (id, \"workspaceId\", \"applicationId\", \"objectMetadataId\", name, \"isUnique\", \
              \"indexType\", \"universalIdentifier\") \
             VALUES ($1, $2, $3, $4, $5, false, 'BTREE', $6)",
        )
        .bind(index_id)
        .bind(workspace.workspace_id)
        .bind(application_id.0)
        .bind(object_id)
        .bind(index_name)
        .bind(Uuid::new_v4())
        .execute(&state.core_pool)
        .await
        .expect("inserting the index metadata");

        sqlx::query(
            "INSERT INTO core.\"indexFieldMetadata\" \
             (\"indexMetadataId\", \"fieldMetadataId\", \"order\", \"workspaceId\") \
             VALUES ($1, $2, 0, $3)",
        )
        .bind(index_id)
        .bind(field_id)
        .bind(workspace.workspace_id)
        .execute(&state.core_pool)
        .await
        .expect("inserting the index field metadata");

        index_id
    }

    async fn persist_deferred(
        state: &AppState,
        workspace_id: Uuid,
        application_universal_identifier: &str,
        name: &str,
        payload: Value,
    ) {
        let mut transaction = state.core_pool.begin().await.expect("tx");

        persist_many(
            &mut transaction,
            workspace_id,
            &[DeferredOperationInput {
                name: name.to_string(),
                payload,
                application_universal_identifier: application_universal_identifier.to_string(),
            }],
        )
        .await
        .expect("persisting the deferred operation");

        transaction.commit().await.expect("committing");
    }

    async fn deferred_row(state: &AppState, workspace_id: Uuid) -> DeferredWorkspaceOperation {
        sqlx::query_as(
            "SELECT id, \"workspaceId\", \"applicationUniversalIdentifier\", name, payload, \
             position, \"runByVersion\", status, attempts, \"lastError\" \
             FROM core.\"deferredWorkspaceOperation\" WHERE \"workspaceId\" = $1",
        )
        .bind(workspace_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("reading the deferred row")
    }

    async fn deferred_count(state: &AppState, workspace_id: Uuid) -> i64 {
        let row: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM core.\"deferredWorkspaceOperation\" WHERE \"workspaceId\" = $1",
        )
        .bind(workspace_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("counting deferred rows");

        row.0
    }

    async fn index_exists(state: &AppState, schema: &str, index_name: &str) -> bool {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM pg_indexes WHERE schemaname = $1 AND indexname = $2)",
        )
        .bind(schema)
        .bind(index_name)
        .fetch_one(&state.core_pool)
        .await
        .expect("checking the index");

        row.0
    }

    fn object_create_migration(workspace: &TestWorkspace) -> WorkspaceMigration {
        WorkspaceMigration {
            actions: vec![UniversalAction::Create(UniversalCreateAction {
                metadata_name: MetadataName::ObjectMetadata,
                flat_entity: Some(UniversalFlatEntity::ObjectMetadata(Box::new(
                    UniversalFlatObjectMetadata {
                        universal_identifier: Uuid::new_v4().to_string(),
                        application_universal_identifier: workspace
                            .application_universal_identifier
                            .clone(),
                        name_singular: "gatewidget".to_string(),
                        name_plural: "gatewidgets".to_string(),
                        label_singular: "GateWidget".to_string(),
                        label_plural: "GateWidgets".to_string(),
                        is_active: true,
                        target_table_name: Some("_gatewidget".to_string()),
                        ..UniversalFlatObjectMetadata::default()
                    },
                ))),
                ..UniversalCreateAction::default()
            })],
            application_universal_identifier: workspace.application_universal_identifier.clone(),
        }
    }

    #[tokio::test]
    async fn deferred_build_index_is_claimed_executed_and_deleted() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8widget").await;
        let object_id = insert_object_metadata(&state, &workspace, "_u8widget").await;
        let field_id = insert_text_field(&state, &workspace, object_id, "codeText").await;
        let index_id =
            insert_deferrable_index(&state, &workspace, object_id, field_id, "IDX_u8_code").await;

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            BUILD_INDEX,
            serde_json::json!({ "indexMetadataId": index_id.to_string() }),
        )
        .await;

        assert!(
            has_in_flight_schema_affecting(&state, workspace.workspace_id)
                .await
                .expect("checking in-flight operations"),
            "a pending build_index must be considered in flight"
        );

        let claimed = claim_pending(&state, workspace.workspace_id, 10)
            .await
            .expect("claiming the pending operation");

        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].status, "IN_PROGRESS");
        assert_eq!(claimed[0].attempts, 1);

        execute_and_record(&state, workspace.workspace_id, &claimed[0])
            .await
            .expect("executing the claimed operation");

        assert!(
            index_exists(&state, &workspace.schema_name, "IDX_u8_code").await,
            "the deferred index must be built"
        );
        assert_eq!(
            deferred_count(&state, workspace.workspace_id).await,
            0,
            "the row must be deleted on success like upstream"
        );
        assert!(
            !has_in_flight_schema_affecting(&state, workspace.workspace_id)
                .await
                .expect("rechecking in-flight operations"),
            "deleting the row clears the in-flight gate"
        );
    }

    #[tokio::test]
    async fn run_pending_inline_executes_every_pending_row() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8inline").await;
        let object_id = insert_object_metadata(&state, &workspace, "_u8inline").await;
        let field_id = insert_text_field(&state, &workspace, object_id, "codeText").await;
        let index_id =
            insert_deferrable_index(&state, &workspace, object_id, field_id, "IDX_u8_inline").await;

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            BUILD_INDEX,
            serde_json::json!({ "indexMetadataId": index_id.to_string() }),
        )
        .await;

        let executed = run_pending_inline(&state, workspace.workspace_id)
            .await
            .expect("running pending operations inline");

        assert_eq!(executed, 1);
        assert!(index_exists(&state, &workspace.schema_name, "IDX_u8_inline").await);
        assert_eq!(deferred_count(&state, workspace.workspace_id).await, 0);
    }

    #[tokio::test]
    async fn a_failing_operation_returns_to_pending_and_fails_at_the_limit() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8failing").await;

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            "unsupported_operation",
            serde_json::json!({}),
        )
        .await;

        for attempt in 1..=MAX_ATTEMPTS {
            let _ = run_pending_inline(&state, workspace.workspace_id).await;

            let row = deferred_row(&state, workspace.workspace_id).await;

            assert_eq!(row.attempts, attempt, "the claim increments attempts");
            assert!(
                row.last_error
                    .as_deref()
                    .unwrap_or_default()
                    .contains("unsupported deferred operation"),
                "the failure reason must be recorded"
            );

            if attempt < MAX_ATTEMPTS {
                assert_eq!(
                    row.status, "PENDING",
                    "a failure below the limit goes back to PENDING"
                );
            } else {
                assert_eq!(
                    row.status, "FAILED",
                    "the final attempt parks the row in FAILED"
                );
            }
        }

        assert_eq!(
            run_pending_inline(&state, workspace.workspace_id)
                .await
                .expect("nothing left to claim"),
            0
        );
        assert_eq!(
            deferred_row(&state, workspace.workspace_id).await.status,
            "FAILED"
        );

        let reset = reset_failed(&state, workspace.workspace_id)
            .await
            .expect("resetting failed operations");

        assert_eq!(reset, 1);

        let row = deferred_row(&state, workspace.workspace_id).await;
        assert_eq!(row.status, "PENDING");
        assert_eq!(row.attempts, 0);
    }

    #[tokio::test]
    async fn in_flight_schema_affecting_operations_block_new_migrations() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8gate").await;

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            REBUILD_OBJECT,
            serde_json::json!({ "objectMetadataId": Uuid::new_v4().to_string() }),
        )
        .await;

        let migration = object_create_migration(&workspace);

        let error = run_workspace_migration(
            &state,
            RunnerInput {
                workspace_id: workspace.workspace_id,
                migration: &migration,
                should_defer_actions: true,
            },
        )
        .await
        .expect_err("a schema-affecting run must be blocked while one is in flight");

        assert_eq!(
            error.code,
            "DEFERRED_WORKSPACE_MIGRATION_ACTIONS_IN_PROGRESS"
        );

        let executed = run_pending_inline(&state, workspace.workspace_id)
            .await
            .expect("inline pass is best effort");

        assert_eq!(executed, 0);

        let row = deferred_row(&state, workspace.workspace_id).await;
        assert_eq!(row.status, "PENDING");

        assert!(
            has_in_flight_schema_affecting(&state, workspace.workspace_id)
                .await
                .expect("rechecking the gate"),
            "the rescheduled row still gates new schema-affecting migrations"
        );

        sqlx::query("DELETE FROM core.\"deferredWorkspaceOperation\" WHERE \"workspaceId\" = $1")
            .bind(workspace.workspace_id)
            .execute(&state.core_pool)
            .await
            .expect("clearing the row");

        assert!(
            !has_in_flight_schema_affecting(&state, workspace.workspace_id)
                .await
                .expect("rechecking the gate after the drain"),
            "an empty deferred queue must un-block schema-affecting migrations"
        );
    }

    #[tokio::test]
    async fn claim_backoff_holds_retry_rows_until_the_delay_elapses() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8backoff").await;

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            BUILD_INDEX,
            serde_json::json!({ "indexMetadataId": Uuid::new_v4().to_string() }),
        )
        .await;

        sqlx::query(
            "UPDATE core.\"deferredWorkspaceOperation\" SET attempts = 1 WHERE \"workspaceId\" = $1",
        )
        .bind(workspace.workspace_id)
        .execute(&state.core_pool)
        .await
        .expect("marking the row as a retry");

        let held = claim_pending_backoff(&state, workspace.workspace_id, 10)
            .await
            .expect("claiming a fresh retry row");

        assert!(
            held.is_empty(),
            "a retry row inside its thirty second window must wait"
        );

        let immediate = claim_pending(&state, workspace.workspace_id, 10)
            .await
            .expect("immediate claim ignores the queue backoff");

        assert_eq!(immediate.len(), 1, "inline seeding is never held back");

        execute_and_record(&state, workspace.workspace_id, &immediate[0])
            .await
            .expect("the missing index metadata makes the handler a no-op success");

        assert_eq!(
            deferred_count(&state, workspace.workspace_id).await,
            0,
            "success deletes the row"
        );

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            BUILD_INDEX,
            serde_json::json!({ "indexMetadataId": Uuid::new_v4().to_string() }),
        )
        .await;

        sqlx::query(
            "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'PENDING', attempts = 1, \
             \"updatedAt\" = now() - make_interval(secs => 31) WHERE \"workspaceId\" = $1",
        )
        .bind(workspace.workspace_id)
        .execute(&state.core_pool)
        .await
        .expect("aging the row past its backoff window");

        let aged = claim_pending_backoff(&state, workspace.workspace_id, 10)
            .await
            .expect("claiming the aged row");

        assert_eq!(aged.len(), 1, "the aged row is claimable again");
        assert_eq!(aged[0].attempts, 2);
    }

    #[tokio::test]
    async fn concurrent_claims_partition_the_pending_rows() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8concurrent").await;

        for position in 0..4 {
            persist_deferred(
                &state,
                workspace.workspace_id,
                &workspace.application_universal_identifier,
                "unsupported_operation",
                serde_json::json!({ "position": position }),
            )
            .await;
        }

        let (left, right) = tokio::join!(
            claim_pending(&state, workspace.workspace_id, 2),
            claim_pending(&state, workspace.workspace_id, 2)
        );

        let left = left.expect("left claim");
        let right = right.expect("right claim");

        assert_eq!(
            left.len() + right.len(),
            4,
            "SKIP LOCKED must split the rows"
        );

        let right_ids: Vec<Uuid> = right.iter().map(|row| row.id).collect();

        assert!(
            left.iter().all(|row| !right_ids.contains(&row.id)),
            "no row may be claimed twice"
        );
    }

    #[tokio::test]
    async fn stale_in_progress_rows_are_recovered() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8stale").await;

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            BUILD_INDEX,
            serde_json::json!({ "indexMetadataId": Uuid::new_v4().to_string() }),
        )
        .await;

        let stale_age_ms = DEFERRED_STALE_THRESHOLD_MS + 60_000;

        sqlx::query(
            "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'IN_PROGRESS', attempts = 1, \
             \"startedAt\" = now() - $2 * interval '1 millisecond' WHERE \"workspaceId\" = $1",
        )
        .bind(workspace.workspace_id)
        .bind(stale_age_ms)
        .execute(&state.core_pool)
        .await
        .expect("ageing the row past the stale threshold");

        let recovered = reset_stale_in_progress(&state)
            .await
            .expect("recovering stale rows");

        assert!(recovered >= 1);

        let row = deferred_row(&state, workspace.workspace_id).await;
        assert_eq!(row.status, "PENDING");
        assert_eq!(row.last_error.as_deref(), Some(STALE_IN_PROGRESS_ERROR));

        sqlx::query(
            "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'IN_PROGRESS', attempts = 3, \
             \"startedAt\" = now() - $2 * interval '1 millisecond' WHERE \"workspaceId\" = $1",
        )
        .bind(workspace.workspace_id)
        .bind(stale_age_ms)
        .execute(&state.core_pool)
        .await
        .expect("ageing a final-attempt row");

        reset_stale_in_progress(&state)
            .await
            .expect("recovering again");

        let row = deferred_row(&state, workspace.workspace_id).await;
        assert_eq!(
            row.status, "FAILED",
            "a stale final attempt fails like upstream"
        );
    }

    #[tokio::test]
    async fn validate_foreign_key_finalises_the_constraint() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8parent").await;

        sqlx::query(&format!(
            "CREATE TABLE {}.\"_u8child\" (id uuid PRIMARY KEY DEFAULT public.uuid_generate_v4(), \
             \"ownerId\" uuid)",
            quoted_identifier(&workspace.schema_name).unwrap()
        ))
        .execute(&state.core_pool)
        .await
        .expect("creating the child table");

        let parent_object_id = insert_object_metadata(&state, &workspace, "_u8parent").await;
        let child_object_id = insert_object_metadata(&state, &workspace, "_u8child").await;

        let application_id: (Uuid,) =
            sqlx::query_as("SELECT id FROM core.\"application\" WHERE \"workspaceId\" = $1")
                .bind(workspace.workspace_id)
                .fetch_one(&state.core_pool)
                .await
                .expect("reading the application");

        let field_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO core.\"fieldMetadata\" \
             (id, \"workspaceId\", \"applicationId\", \"objectMetadataId\", name, label, type, \
              \"isActive\", \"universalIdentifier\", \"relationTargetObjectMetadataId\") \
             VALUES ($1, $2, $3, $4, 'owner', 'Owner', 'RELATION', true, $5, $6)",
        )
        .bind(field_id)
        .bind(workspace.workspace_id)
        .bind(application_id.0)
        .bind(child_object_id)
        .bind(Uuid::new_v4())
        .bind(parent_object_id)
        .execute(&state.core_pool)
        .await
        .expect("inserting the relation field");

        let definition = WorkspaceSchemaForeignKeyDefinition {
            table_name: "_u8child".to_string(),
            column_name: "ownerId".to_string(),
            referenced_table_name: "_u8parent".to_string(),
            referenced_column_name: "id".to_string(),
            on_delete: Some("CASCADE".to_string()),
            on_update: None,
        };

        let mut connection = state.core_pool.acquire().await.expect("connection");

        foreign_key::create_foreign_key(&mut connection, &workspace.schema_name, &definition, true)
            .await
            .expect("creating the not-valid constraint");

        drop(connection);

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            VALIDATE_FOREIGN_KEY,
            serde_json::json!({ "fieldMetadataId": field_id.to_string() }),
        )
        .await;

        let executed = run_pending_inline(&state, workspace.workspace_id)
            .await
            .expect("validating the foreign key");

        assert_eq!(executed, 1);
        assert_eq!(deferred_count(&state, workspace.workspace_id).await, 0);

        let validated: (bool,) = sqlx::query_as(
            "SELECT con.convalidated FROM pg_constraint con \
             JOIN pg_namespace n ON n.oid = con.connamespace \
             WHERE n.nspname = $1 AND con.conname = $2",
        )
        .bind(&workspace.schema_name)
        .bind(foreign_key::foreign_key_constraint_name(&definition))
        .fetch_one(&state.core_pool)
        .await
        .expect("reading the constraint");

        assert!(validated.0, "the constraint must be validated");
    }

    #[tokio::test]
    async fn rebuild_object_rewrites_the_table_only_with_dropped_columns() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8rebuild").await;
        let object_id = insert_object_metadata(&state, &workspace, "_u8rebuild").await;
        insert_text_field(&state, &workspace, object_id, "codeText").await;

        sqlx::query(&format!(
            "ALTER TABLE {}.\"_u8rebuild\" ADD COLUMN \"extraText\" text",
            quoted_identifier(&workspace.schema_name).unwrap()
        ))
        .execute(&state.core_pool)
        .await
        .expect("adding the dropped column");

        sqlx::query(&format!(
            "INSERT INTO {}.\"_u8rebuild\" (\"codeText\") VALUES ('kept')",
            quoted_identifier(&workspace.schema_name).unwrap()
        ))
        .execute(&state.core_pool)
        .await
        .expect("inserting a row");

        sqlx::query(&format!(
            "ALTER TABLE {}.\"_u8rebuild\" DROP COLUMN \"extraText\"",
            quoted_identifier(&workspace.schema_name).unwrap()
        ))
        .execute(&state.core_pool)
        .await
        .expect("dropping the column");

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            REBUILD_OBJECT,
            serde_json::json!({ "objectMetadataId": object_id.to_string() }),
        )
        .await;

        let executed = run_pending_inline(&state, workspace.workspace_id)
            .await
            .expect("rebuilding the object");

        assert_eq!(executed, 1, "a table with dropped columns must be rebuilt");
        assert_eq!(deferred_count(&state, workspace.workspace_id).await, 0);

        let preserved: (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {}.\"_u8rebuild\" WHERE \"codeText\" = 'kept'",
            quoted_identifier(&workspace.schema_name).unwrap()
        ))
        .fetch_one(&state.core_pool)
        .await
        .expect("counting preserved rows");

        assert_eq!(preserved.0, 1);

        let rebuilt_left: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
             WHERE table_schema = $1 AND table_name = '_u8rebuild__rebuild')",
        )
        .bind(&workspace.schema_name)
        .fetch_one(&state.core_pool)
        .await
        .expect("checking the interim table");

        assert!(!rebuilt_left.0, "the interim table must be gone");

        sqlx::query("DELETE FROM core.\"deferredWorkspaceOperation\" WHERE \"workspaceId\" = $1")
            .bind(workspace.workspace_id)
            .execute(&state.core_pool)
            .await
            .expect("clearing rows");

        let table_oid_before: (i64,) = sqlx::query_as(
            "SELECT c.oid::bigint FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname = $1 AND c.relname = $2",
        )
        .bind(&workspace.schema_name)
        .bind("_u8rebuild")
        .fetch_one(&state.core_pool)
        .await
        .expect("reading the table oid");

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            REBUILD_OBJECT,
            serde_json::json!({ "objectMetadataId": object_id.to_string() }),
        )
        .await;

        let executed = run_pending_inline(&state, workspace.workspace_id)
            .await
            .expect("skipping a clean rebuild");

        assert_eq!(executed, 1, "a skip still counts as a success");

        let table_oid_after: (i64,) = sqlx::query_as(
            "SELECT c.oid::bigint FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname = $1 AND c.relname = $2",
        )
        .bind(&workspace.schema_name)
        .bind("_u8rebuild")
        .fetch_one(&state.core_pool)
        .await
        .expect("reading the table oid again");

        assert_eq!(
            table_oid_before.0, table_oid_after.0,
            "a table without dropped column slots must not be rewritten"
        );
    }

    #[tokio::test]
    async fn the_queue_pass_drains_every_workspace() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let mut workspaces = Vec::new();

        for table in ["_u8q1", "_u8q2"] {
            let workspace = create_test_workspace(&state, table).await;
            let object_id = insert_object_metadata(&state, &workspace, table).await;
            let field_id = insert_text_field(&state, &workspace, object_id, "codeText").await;
            let index_id = insert_deferrable_index(
                &state,
                &workspace,
                object_id,
                field_id,
                &format!("IDX_u8_{}", table.trim_start_matches("_u8")),
            )
            .await;

            persist_deferred(
                &state,
                workspace.workspace_id,
                &workspace.application_universal_identifier,
                BUILD_INDEX,
                serde_json::json!({ "indexMetadataId": index_id.to_string() }),
            )
            .await;

            workspaces.push(workspace);
        }

        let executed = crate::workspace_migrations::worker::run_queue_pass(&state)
            .await
            .expect("running the queue pass");

        assert!(
            executed >= 2,
            "one pass must drain the pending rows of every workspace"
        );

        for workspace in &workspaces {
            assert_eq!(
                deferred_count(&state, workspace.workspace_id).await,
                0,
                "the pass must delete the rows it executed"
            );
        }
    }

    #[tokio::test]
    async fn the_worker_loop_stops_on_the_shutdown_watch() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state, "_u8worker").await;
        let object_id = insert_object_metadata(&state, &workspace, "_u8worker").await;
        let field_id = insert_text_field(&state, &workspace, object_id, "codeText").await;
        let index_id =
            insert_deferrable_index(&state, &workspace, object_id, field_id, "IDX_u8_worker").await;

        persist_deferred(
            &state,
            workspace.workspace_id,
            &workspace.application_universal_identifier,
            BUILD_INDEX,
            serde_json::json!({ "indexMetadataId": index_id.to_string() }),
        )
        .await;

        let (shutdown, handle) =
            crate::workspace_migrations::worker::spawn_deferred_worker_with_shutdown(
                state.clone(),
                Duration::from_millis(20),
            );

        let mut observed_deleted = false;

        for _ in 0..400 {
            if deferred_count(&state, workspace.workspace_id).await == 0 {
                observed_deleted = true;
                break;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        shutdown.shutdown();

        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("the worker must exit on shutdown")
            .expect("the worker task must not panic");

        assert!(observed_deleted);
        assert!(index_exists(&state, &workspace.schema_name, "IDX_u8_worker").await);
        assert_eq!(deferred_count(&state, workspace.workspace_id).await, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_affecting_names_match_the_upstream_set() {
        assert!(is_schema_affecting_name("build_index"));
        assert!(is_schema_affecting_name("validate_foreignKey"));
        assert!(is_schema_affecting_name("rebuild_object"));
        assert!(!is_schema_affecting_name("compute_storedFormulaFields"));
    }

    #[test]
    fn statuses_use_the_upstream_strings() {
        assert_eq!(DeferredStatus::Pending.as_str(), "PENDING");
        assert_eq!(DeferredStatus::InProgress.as_str(), "IN_PROGRESS");
        assert_eq!(DeferredStatus::Failed.as_str(), "FAILED");
    }

    #[test]
    fn retry_limit_matches_three_attempts() {
        assert_eq!(MAX_ATTEMPTS, 3);
    }

    #[test]
    fn failure_below_the_limit_returns_to_pending() {
        assert_eq!(status_after_failure(1), DeferredStatus::Pending);
        assert_eq!(status_after_failure(2), DeferredStatus::Pending);
    }

    #[test]
    fn failure_at_or_above_the_limit_is_failed() {
        assert_eq!(status_after_failure(MAX_ATTEMPTS), DeferredStatus::Failed);
        assert_eq!(status_after_failure(4), DeferredStatus::Failed);
    }

    #[test]
    fn backoff_doubles_from_the_initial_thirty_seconds() {
        assert_eq!(next_attempt_backoff_ms(0), 0);
        assert_eq!(next_attempt_backoff_ms(1), 30_000);
        assert_eq!(next_attempt_backoff_ms(2), 60_000);
        assert_eq!(next_attempt_backoff_ms(3), 120_000);
    }

    #[test]
    fn backoff_saturates_for_absurd_attempt_counts() {
        assert_eq!(next_attempt_backoff_ms(i32::MAX), 30_000u64 << 20);
    }

    #[test]
    fn error_messages_are_capped_for_the_text_column() {
        let long = "x".repeat(5_000);
        let truncated = truncate_error_message(&long);

        assert!(truncated.len() <= 3903);
        assert!(truncated.ends_with('…'));

        let short = "boom";
        assert_eq!(truncate_error_message(short), "boom");
    }

    #[test]
    fn index_columns_follow_the_upstream_field_rules() {
        let scalar = "codeText".to_string();
        let relation = "owner".to_string();
        let emails = "emails".to_string();

        let fields = vec![
            (None, scalar.clone(), "TEXT".to_string()),
            (None, relation.clone(), "RELATION".to_string()),
            (None, emails.clone(), "EMAILS".to_string()),
            (
                Some("primaryEmail".to_string()),
                emails.clone(),
                "EMAILS".to_string(),
            ),
        ];

        let columns = index_column_names_for_fields(&fields);

        assert_eq!(
            columns,
            vec![
                scalar,
                compute_relation_join_column_name(&relation),
                compute_composite_column_name(&emails, "primaryEmail"),
                compute_composite_column_name(&emails, "primaryEmail"),
            ]
        );
    }
}
