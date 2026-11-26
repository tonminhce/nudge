#![allow(dead_code)]

use serde_json::Value;
use sqlx::PgConnection;
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use super::ddl::foreign_key;
use super::ddl::foreign_key::WorkspaceSchemaForeignKeyDefinition;
use super::ddl::index;
use super::ddl::index::WorkspaceSchemaIndexDefinition;
use super::runner::object_table_name;
use super::runner::resolve_workspace_runtime;
use super::runner::RunnerError;
use super::runner::WorkspaceRuntime;
use crate::metadata::names::compute_relation_join_column_name;
use crate::state::AppState;

pub const BUILD_INDEX: &str = "build_index";
pub const VALIDATE_FOREIGN_KEY: &str = "validate_foreignKey";
pub const REBUILD_OBJECT: &str = "rebuild_object";

pub const SCHEMA_AFFECTING_DEFERRED_OPERATIONS: [&str; 3] =
    [BUILD_INDEX, VALIDATE_FOREIGN_KEY, REBUILD_OBJECT];

pub const MAX_ATTEMPTS: i32 = 3;
pub const DEFAULT_RUN_BY_VERSION: &str = "nudge-0.1.0";

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
    Done,
    Failed,
}

impl DeferredStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            DeferredStatus::Pending => "PENDING",
            DeferredStatus::InProgress => "IN_PROGRESS",
            DeferredStatus::Done => "DONE",
            DeferredStatus::Failed => "FAILED",
        }
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
        "SELECT COALESCE(MAX(position), -1) + 1 FROM core.\"deferredWorkspaceOperation\" \
         WHERE \"workspaceId\" = $1",
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
    let rows: Vec<DeferredWorkspaceOperation> = sqlx::query_as(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'IN_PROGRESS', \
         \"startedAt\" = now(), attempts = attempts + 1, \"updatedAt\" = now() \
         WHERE id IN ( \
             SELECT id FROM core.\"deferredWorkspaceOperation\" \
             WHERE \"workspaceId\" = $1 AND status = 'PENDING' \
             ORDER BY position ASC, \"createdAt\" ASC LIMIT $2 \
             FOR UPDATE SKIP LOCKED \
         ) RETURNING *",
    )
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(&state.core_pool)
    .await?;

    Ok(rows)
}

pub async fn run_pending_inline(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<usize, RunnerError> {
    let mut executed = 0;

    loop {
        let claimed = claim_pending(state, workspace_id, 10).await?;

        if claimed.is_empty() {
            break;
        }

        for operation in claimed {
            execute_and_record(state, workspace_id, &operation).await?;
            executed += 1;
        }
    }

    Ok(executed)
}

pub async fn execute_and_record(
    state: &AppState,
    workspace_id: Uuid,
    operation: &DeferredWorkspaceOperation,
) -> Result<(), RunnerError> {
    match execute(state, workspace_id, operation).await {
        Ok(()) => {
            mark_done(state, operation.id).await?;
            Ok(())
        }
        Err(error) => {
            if operation.attempts >= MAX_ATTEMPTS {
                mark_failed(state, operation.id, &error.to_string()).await?;
            } else {
                mark_pending_again(state, operation.id, &error.to_string()).await?;
            }

            Err(error)
        }
    }
}

async fn mark_done(state: &AppState, operation_id: Uuid) -> Result<(), RunnerError> {
    sqlx::query(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'DONE', \
         \"lastError\" = NULL, \"updatedAt\" = now() WHERE id = $1",
    )
    .bind(operation_id)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

async fn mark_failed(
    state: &AppState,
    operation_id: Uuid,
    last_error: &str,
) -> Result<(), RunnerError> {
    sqlx::query(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'FAILED', \
         \"lastError\" = $2, \"updatedAt\" = now() WHERE id = $1",
    )
    .bind(operation_id)
    .bind(last_error)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

async fn mark_pending_again(
    state: &AppState,
    operation_id: Uuid,
    last_error: &str,
) -> Result<(), RunnerError> {
    sqlx::query(
        "UPDATE core.\"deferredWorkspaceOperation\" SET status = 'PENDING', \
         \"lastError\" = $2, \"updatedAt\" = now() WHERE id = $1",
    )
    .bind(operation_id)
    .bind(last_error)
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

pub async fn execute(
    state: &AppState,
    workspace_id: Uuid,
    operation: &DeferredWorkspaceOperation,
) -> Result<(), RunnerError> {
    let runtime = resolve_workspace_runtime(
        &state.core_pool,
        workspace_id,
        &operation.application_universal_identifier,
    )
    .await?;

    let mut transaction = state.core_pool.begin().await?;

    let result = match operation.name.as_str() {
        BUILD_INDEX => execute_build_index(&mut transaction, &runtime, &operation.payload).await,
        VALIDATE_FOREIGN_KEY => {
            execute_validate_foreign_key(&mut transaction, &runtime, &operation.payload).await
        }
        REBUILD_OBJECT => {
            execute_rebuild_object(&mut transaction, &runtime, &operation.payload).await
        }
        other => Err(RunnerError::execution_failed(format!(
            "unsupported deferred operation {other}"
        ))),
    };

    match result {
        Ok(()) => {
            transaction.commit().await?;
            Ok(())
        }
        Err(error) => {
            transaction.rollback().await.ok();
            Err(error)
        }
    }
}

async fn execute_build_index(
    transaction: &mut Transaction<'_, Postgres>,
    runtime: &WorkspaceRuntime,
    payload: &Value,
) -> Result<(), RunnerError> {
    let connection: &mut PgConnection = transaction;

    let index_metadata_id = payload
        .get("indexMetadataId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunnerError::execution_failed("build_index payload needs indexMetadataId")
        })?;

    let index_metadata_id = Uuid::parse_str(index_metadata_id)
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let row: Option<(String, Uuid, bool, Option<String>, String)> = sqlx::query_as(
        "SELECT name, \"objectMetadataId\", \"isUnique\", \"indexWhereClause\", \"indexType\" \
         FROM core.\"indexMetadata\" WHERE id = $1",
    )
    .bind(index_metadata_id)
    .fetch_optional(&mut *connection)
    .await?;

    let Some((name, object_id, is_unique, where_clause, index_type)) = row else {
        return Ok(());
    };

    let table_name = object_table_name(&mut *connection, object_id).await?;

    let columns: Vec<(String,)> = sqlx::query_as(
        "SELECT f.name FROM core.\"indexMetadata\" i \
         JOIN core.\"indexFieldMetadata\" ifm ON ifm.\"indexMetadataId\" = i.id \
         JOIN core.\"fieldMetadata\" f ON f.id = ifm.\"fieldMetadataId\" \
         WHERE i.id = $1 ORDER BY ifm.\"order\" ASC",
    )
    .bind(index_metadata_id)
    .fetch_all(&mut *connection)
    .await?;

    let mut column_names: Vec<String> = Vec::new();

    for (column_name,) in columns {
        let field_lookup: Vec<(String, String)> = sqlx::query_as(
            "SELECT f.name, f.type FROM core.\"fieldMetadata\" f \
             JOIN core.\"indexFieldMetadata\" ifm ON ifm.\"fieldMetadataId\" = f.id \
             WHERE ifm.\"indexMetadataId\" = $1",
        )
        .bind(index_metadata_id)
        .fetch_all(&mut *connection)
        .await?;

        let mut resolved = column_name;

        for (name, field_type) in &field_lookup {
            if name == &resolved && field_type == "RELATION" {
                resolved = compute_relation_join_column_name(name);
            }
        }

        column_names.push(resolved);
    }

    let definition = WorkspaceSchemaIndexDefinition {
        name,
        columns: column_names,
        is_unique,
        index_type: Some(index_type),
        where_clause,
    };

    index::create_index_concurrently(
        &mut *connection,
        &runtime.schema_name,
        &table_name,
        &definition,
    )
    .await?;

    Ok(())
}

async fn execute_validate_foreign_key(
    transaction: &mut Transaction<'_, Postgres>,
    runtime: &WorkspaceRuntime,
    payload: &Value,
) -> Result<(), RunnerError> {
    let connection: &mut PgConnection = transaction;

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
    .fetch_optional(&mut *connection)
    .await?;

    let Some((object_id, field_name, target_object_id)) = row else {
        return Ok(());
    };

    let Some(target_object_id) = target_object_id else {
        return Ok(());
    };

    let table_name = object_table_name(&mut *connection, object_id).await?;
    let target_table = object_table_name(&mut *connection, target_object_id).await?;

    let foreign_key = WorkspaceSchemaForeignKeyDefinition {
        table_name,
        column_name: format!("{field_name}Id"),
        referenced_table_name: target_table,
        referenced_column_name: "id".to_string(),
        on_delete: Some("CASCADE".to_string()),
        on_update: None,
    };

    foreign_key::validate_foreign_key(&mut *connection, &runtime.schema_name, &foreign_key).await?;

    Ok(())
}

async fn execute_rebuild_object(
    transaction: &mut Transaction<'_, Postgres>,
    runtime: &WorkspaceRuntime,
    payload: &Value,
) -> Result<(), RunnerError> {
    let connection: &mut PgConnection = transaction;

    let object_metadata_id = payload
        .get("objectMetadataId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunnerError::execution_failed("rebuild_object payload needs objectMetadataId")
        })?;

    let object_id = Uuid::parse_str(object_metadata_id)
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let table_name = object_table_name(&mut *connection, object_id).await?;

    let rebuilt_table_name = format!("{table_name}__rebuild");

    let table_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables          WHERE table_schema = $1 AND table_name = $2)",
    )
    .bind(&runtime.schema_name)
    .bind(&table_name)
    .fetch_one(&mut *connection)
    .await?;

    if !table_exists.0 {
        return Ok(());
    }

    sqlx::query(&format!(
        "ALTER TABLE {} RENAME TO {}",
        quoted(&runtime.schema_name, &table_name)?,
        quoted_identifier(&rebuilt_table_name)?
    ))
    .execute(&mut *connection)
    .await?;

    let definitions =
        regenerated_column_definitions(connection, &runtime.schema_name, object_id, &table_name)
            .await?;

    super::ddl::table::create_table(
        &mut *connection,
        &runtime.schema_name,
        &table_name,
        &definitions,
    )
    .await?;

    let columns: Vec<(String,)> = sqlx::query_as(
        "SELECT column_name FROM information_schema.columns          WHERE table_schema = $1 AND table_name = $2 AND column_name NOT IN ( \
             SELECT column_name FROM information_schema.columns \
             WHERE table_schema = $1 AND table_name = $3 \
         )",
    )
    .bind(&runtime.schema_name)
    .bind(&table_name)
    .bind(&rebuilt_table_name)
    .fetch_all(&mut *connection)
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
    .fetch_all(&mut *connection)
    .await?;

    let _ = columns;

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
        .execute(&mut *connection)
        .await?;
    }

    super::ddl::table::drop_table(
        &mut *connection,
        &runtime.schema_name,
        &rebuilt_table_name,
        true,
    )
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
         WHERE \"objectMetadataId\" = $1 AND \"deletedAt\" IS NULL ORDER BY \"createdAt\" ASC",
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
        assert_eq!(DeferredStatus::Done.as_str(), "DONE");
        assert_eq!(DeferredStatus::Failed.as_str(), "FAILED");
    }

    #[test]
    fn retry_limit_matches_three_attempts() {
        assert_eq!(MAX_ATTEMPTS, 3);
    }
}
