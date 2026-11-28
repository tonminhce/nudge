use serde_json::Map;
use serde_json::Value;
use sqlx::Column;
use sqlx::PgConnection;
use sqlx::Row;
use uuid::Uuid;

use crate::query::filter::Suffixes;
use crate::query::repository::build_column_result_alias;
use crate::query::repository::build_mutation_statement;
use crate::query::repository::quote_identifier;
use crate::query::repository::MutationKind;
use crate::query::repository::MutationStatementState;
use crate::query::repository::MutationWhereClause;

use super::actors::inject_updated_by_on_update;
use super::actors::ActorContext;
use super::base::compile_runner_filter;
use super::base::normalize_record_links;
use super::base::resolve_where_clauses;
use super::base::ObjectModel;
use super::base::RunnerSchema;
use super::permissions::validate_filter_read;
use super::permissions::validate_write;
use super::relation_nested::resolve_relation_nested_queries;
use super::rls::row_write_access_filter;
use super::rls::validate_updated_rows;
use super::snapshots::read_stored_records;
use super::snapshots::readable_stored_columns;
use super::snapshots::snapshot_from_stored_row;
use super::snapshots::stored_column_names;
use super::RunnerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilteredMutationKind {
    Update,
    SoftDelete,
    Destroy,
    Restore,
}

impl FilteredMutationKind {
    pub fn repository_kind(self) -> MutationKind {
        match self {
            Self::Update => MutationKind::Update,
            Self::SoftDelete => MutationKind::SoftDelete,
            Self::Destroy => MutationKind::Delete,
            Self::Restore => MutationKind::Restore,
        }
    }

    pub fn permission_operation(self) -> crate::permissions::ObjectRecordsOperationType {
        match self {
            Self::Update => crate::permissions::ObjectRecordsOperationType::Update,
            Self::SoftDelete | Self::Restore => {
                crate::permissions::ObjectRecordsOperationType::SoftDelete
            }
            Self::Destroy => crate::permissions::ObjectRecordsOperationType::Destroy,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationSnapshot {
    pub before: Value,
    pub after: Option<Value>,
}

#[derive(Debug)]
pub struct MutationExecution {
    pub records: Vec<Value>,
    pub snapshots: Vec<MutationSnapshot>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_mutation(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    kind: FilteredMutationKind,
    filter: Option<Value>,
    data: Option<Value>,
    actor: Option<&ActorContext>,
    with_deleted: bool,
    returning_columns: Vec<String>,
) -> Result<Vec<Value>, RunnerError> {
    Ok(run_mutation_inner(
        pool,
        schema,
        object_name_singular,
        kind,
        filter,
        data,
        actor,
        with_deleted,
        returning_columns,
        false,
        None,
    )
    .await?
    .records)
}

#[allow(clippy::too_many_arguments)]
pub async fn run_mutation_with_snapshots(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    kind: FilteredMutationKind,
    filter: Option<Value>,
    data: Option<Value>,
    actor: Option<&ActorContext>,
    with_deleted: bool,
    returning_columns: Vec<String>,
) -> Result<Vec<MutationSnapshot>, RunnerError> {
    Ok(run_mutation_inner(
        pool,
        schema,
        object_name_singular,
        kind,
        filter,
        data,
        actor,
        with_deleted,
        returning_columns,
        true,
        None,
    )
    .await?
    .snapshots)
}

#[allow(clippy::too_many_arguments)]
pub async fn run_mutation_in_transaction(
    pool: &sqlx::PgPool,
    transaction: &mut PgConnection,
    schema: &RunnerSchema,
    object_name_singular: &str,
    kind: FilteredMutationKind,
    filter: Option<Value>,
    data: Option<Value>,
    actor: Option<&ActorContext>,
    with_deleted: bool,
    returning_columns: Vec<String>,
    capture_snapshots: bool,
) -> Result<MutationExecution, RunnerError> {
    run_mutation_inner(
        pool,
        schema,
        object_name_singular,
        kind,
        filter,
        data,
        actor,
        with_deleted,
        returning_columns,
        capture_snapshots,
        Some(transaction),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_mutation_inner(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    kind: FilteredMutationKind,
    filter: Option<Value>,
    data: Option<Value>,
    actor: Option<&ActorContext>,
    with_deleted: bool,
    returning_columns: Vec<String>,
    capture_snapshots: bool,
    caller_transaction: Option<&mut PgConnection>,
) -> Result<MutationExecution, RunnerError> {
    let model = schema.object(object_name_singular)?;
    let alias = object_name_singular;
    validate_filter_read(schema, model, filter.as_ref())?;
    let mut suffixes = Suffixes::new();
    let compiled_filter =
        compile_runner_filter(schema, object_name_singular, filter.as_ref(), &mut suffixes)?;
    let mut parameter_map = serde_json::Map::new();
    let clauses;

    if let Some(compiled_filter) = compiled_filter {
        clauses = compiled_filter.clauses;
        parameter_map.extend(compiled_filter.params);
    } else {
        clauses = Vec::new();
    }

    let mut user_where = resolve_where_clauses(schema, object_name_singular, alias, &clauses)?;
    let where_clauses = if user_where.is_empty() {
        Vec::new()
    } else {
        if !user_where.starts_with('(') {
            user_where = format!("({user_where})");
        }

        vec![MutationWhereClause::and(user_where.clone())]
    };
    let mut set_columns = Map::new();

    if let Some(data) = data.as_ref().and_then(Value::as_object) {
        set_columns = data.clone();
    }

    if kind == FilteredMutationKind::Update {
        normalize_record_links(model, &mut set_columns);

        if !set_columns.is_empty() {
            let mut relation_records = vec![set_columns];
            resolve_relation_nested_queries(
                pool,
                schema,
                object_name_singular,
                &mut relation_records,
                actor,
            )
            .await?;
            set_columns = relation_records.swap_remove(0);
        }

        inject_updated_by_on_update(model, &mut set_columns, actor)?;
    }

    let expanded_columns = if kind == FilteredMutationKind::Update {
        crate::query::repository::expand_record_columns(&model.shape, &set_columns)
            .map_err(RunnerError::from_repository)?
    } else {
        Vec::new()
    };
    let set_columns = expanded_columns.clone();
    let snapshot_columns = capture_snapshots.then(|| readable_stored_columns(schema, model));
    let returning_columns = if let Some(columns) = &snapshot_columns {
        stored_column_names(columns)
    } else if returning_columns.is_empty() {
        vec![String::from("id")]
    } else {
        returning_columns
    };
    let return_id = returning_columns.iter().any(|column| column == "id");
    let data = set_columns.iter().cloned().collect::<Map<String, Value>>();
    validate_write(
        schema,
        model,
        kind.permission_operation(),
        &data,
        &returning_columns,
    )?;
    let access = row_write_access_filter(
        pool,
        schema,
        object_name_singular,
        kind.permission_operation(),
        &mut suffixes,
    )
    .await?;
    let update_access = if kind == FilteredMutationKind::Update {
        access.clone()
    } else {
        None
    };
    let mut row_access_conditions = Vec::new();
    if let Some(access) = access {
        let condition =
            resolve_where_clauses(schema, object_name_singular, alias, &access.clauses)?;
        row_access_conditions.push(condition);
        parameter_map.extend(access.params);
    }
    let validate_update = update_access.is_some();
    let recompute_access_groups = caller_transaction.is_none()
        && schema.requires_access_group_recompute(object_name_singular);
    let mut returning_columns = returning_columns;
    if (validate_update || recompute_access_groups) && !return_id {
        returning_columns.push(String::from("id"));
    }
    let mut owned_transaction = if caller_transaction.is_none()
        && (validate_update || capture_snapshots || recompute_access_groups)
    {
        let mut transaction = pool.begin().await.map_err(RunnerError::database)?;
        if recompute_access_groups {
            crate::permissions::access_group::lock_company_access_groups(&mut transaction, schema)
                .await
                .map_err(|error| {
                    RunnerError::new("ACCESS_GROUP_RECOMPUTE_FAILED", error.to_string())
                })?;
        }
        Some(transaction)
    } else {
        None
    };
    let mut connection: Option<&mut PgConnection> =
        match (&mut owned_transaction, caller_transaction) {
            (Some(transaction), _) => Some(&mut **transaction),
            (None, Some(transaction)) => Some(transaction),
            (None, None) => None,
        };
    let mut where_clauses = where_clauses;
    let mut parameter_map = parameter_map;
    let mut snapshots = Vec::new();
    if let Some(transaction) = connection.as_mut() {
        let access_condition = row_access_conditions.first().cloned().unwrap_or_default();
        let ids = lock_update_candidates(
            transaction,
            model,
            alias,
            &user_where,
            &access_condition,
            &parameter_map,
            kind,
            with_deleted,
        )
        .await?;
        if capture_snapshots {
            let columns = snapshot_columns.as_ref().expect("snapshot columns");
            let before = read_stored_records(&mut **transaction, model, columns, &ids).await?;
            snapshots = ids
                .iter()
                .filter_map(|id| {
                    before.get(id).cloned().map(|before| MutationSnapshot {
                        before,
                        after: None,
                    })
                })
                .collect();
        }
        if validate_update || capture_snapshots {
            let mut id_parameters = Map::new();
            id_parameters.insert(
                String::from("rlsIds"),
                Value::Array(ids.iter().map(|id| Value::String(id.to_string())).collect()),
            );
            where_clauses = vec![MutationWhereClause::and(format!(
                "{}.{} = ANY(ARRAY[:...rlsIds]::uuid[])",
                quote_identifier(alias)?,
                quote_identifier("id")?
            ))];
            row_access_conditions.clear();
            parameter_map = id_parameters;
        }
    }
    let state = MutationStatementState {
        alias,
        shape: &model.shape,
        kind: kind.repository_kind(),
        set_columns: set_columns.into_iter().collect(),
        where_clauses,
        where_parameters: parameter_map,
        row_access_conditions,
        include_deleted: with_deleted,
        returning_columns,
    };
    let statement = build_mutation_statement(&state).map_err(RunnerError::from_repository)?;
    let mut statement_sql = statement.sql;

    if let Some(columns) = snapshot_columns.as_ref() {
        for column in columns.iter().filter(|column| {
            matches!(
                column.field_metadata_type,
                crate::metadata::flat::FieldMetadataType::Select
                    | crate::metadata::flat::FieldMetadataType::TsVector
            )
        }) {
            let expression = format!(
                "{}::text",
                quote_identifier(alias)? + "." + &quote_identifier(&column.column_name)?
            );
            let returned = format!(
                "{}.{}",
                quote_identifier(alias)?,
                quote_identifier(&column.column_name)?
            );
            statement_sql = statement_sql.replace(&returned, &expression);
        }
    }

    let set_column_names = expanded_columns
        .iter()
        .map(|(column_name, _)| column_name.clone())
        .collect::<Vec<_>>();
    statement_sql = crate::query::repository::cast_set_parameters(
        &statement_sql,
        &model.shape,
        &set_column_names,
    )
    .map_err(RunnerError::from_repository)?;

    let statement = crate::query::repository::MutationStatement {
        sql: statement_sql,
        parameters: statement.parameters,
    };
    let compiled_statement = statement
        .compile()
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let mut query = sqlx::query(&compiled_statement.text);

    for value in &compiled_statement.values {
        query = super::base::bind_json_value(query, value)?;
    }

    let rows = if let Some(transaction) = connection.as_mut() {
        let rows = query
            .fetch_all(&mut **transaction)
            .await
            .map_err(RunnerError::database)?;
        let id_alias = build_column_result_alias(alias, "id");
        let ids = rows
            .iter()
            .map(|row| row.try_get::<Uuid, _>(id_alias.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(RunnerError::database)?;
        if let Some(update_access) = update_access.as_ref() {
            validate_updated_rows(
                &mut **transaction,
                schema,
                object_name_singular,
                &ids,
                update_access,
            )
            .await?;
        }
        if recompute_access_groups {
            crate::permissions::access_group::recompute_after_write(
                pool,
                transaction,
                schema,
                object_name_singular,
                &ids,
            )
            .await
            .map_err(|error| {
                RunnerError::new("ACCESS_GROUP_RECOMPUTE_FAILED", error.to_string())
            })?;
        }
        rows
    } else {
        query.fetch_all(pool).await.map_err(RunnerError::database)?
    };
    if let Some(transaction) = owned_transaction {
        transaction.commit().await.map_err(RunnerError::database)?;
    }
    if capture_snapshots {
        let columns = snapshot_columns.as_ref().expect("snapshot columns");
        let after = rows
            .iter()
            .map(|row| snapshot_from_stored_row(row, model, columns))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter_map(|record| {
                let id = record
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| Uuid::parse_str(id).ok())?;
                Some((id, record))
            })
            .collect::<std::collections::HashMap<_, _>>();
        if kind != FilteredMutationKind::Destroy {
            for snapshot in &mut snapshots {
                let id = snapshot
                    .before
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| Uuid::parse_str(id).ok());
                snapshot.after = id.and_then(|id| after.get(&id).cloned());
            }
        }
    }
    let mut results = Vec::with_capacity(rows.len());
    let id_alias = build_column_result_alias(alias, "id");

    for row in rows {
        let mut record = Map::new();

        for column in row.columns() {
            if (validate_update || recompute_access_groups)
                && !return_id
                && column.name() == id_alias
            {
                continue;
            }
            let value = super::base::json_cell(&row, column.name())?;
            record.insert(column.name().to_string(), value);
        }

        results.push(Value::Object(record));
    }

    Ok(MutationExecution {
        records: results,
        snapshots,
    })
}

#[allow(clippy::too_many_arguments)]
async fn lock_update_candidates(
    transaction: &mut PgConnection,
    model: &ObjectModel,
    alias: &str,
    user_where: &str,
    access_condition: &str,
    parameters: &Map<String, Value>,
    kind: FilteredMutationKind,
    with_deleted: bool,
) -> Result<Vec<Uuid>, RunnerError> {
    let mut predicates = Vec::new();
    if !user_where.is_empty() {
        predicates.push(format!("({user_where})"));
    }
    if !access_condition.is_empty() {
        predicates.push(format!("({access_condition})"));
    }
    if kind == FilteredMutationKind::SoftDelete && !with_deleted {
        predicates.push(format!(
            "{}.{} IS NULL",
            quote_identifier(alias)?,
            quote_identifier("deletedAt")?
        ));
    }
    let filter_expression = if predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", predicates.join(" AND "))
    };
    let alias_sql = quote_identifier(alias)?;
    let sql = format!(
        "SELECT {alias_sql}.{} FROM {}.{} AS {alias_sql}{filter_expression} ORDER BY {alias_sql}.{} FOR UPDATE OF {alias_sql}",
        quote_identifier("id")?,
        quote_identifier(&model.shape.schema_name)?,
        quote_identifier(&model.shape.table_name)?,
        quote_identifier("id")?,
    );
    let compiled = crate::query::compile::compile_named_parameters(&sql, parameters)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let mut query = sqlx::query(&compiled.text);
    for value in &compiled.values {
        query = super::base::bind_json_value(query, value)?;
    }
    let rows = query
        .fetch_all(&mut *transaction)
        .await
        .map_err(RunnerError::database)?;
    rows.iter()
        .map(|row| row.try_get::<Uuid, _>(0))
        .collect::<Result<Vec<_>, _>>()
        .map_err(RunnerError::database)
}

#[allow(clippy::too_many_arguments)]
pub async fn run_mutation_one(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    kind: FilteredMutationKind,
    filter: Option<Value>,
    data: Option<Value>,
    actor: Option<&ActorContext>,
    with_deleted: bool,
    returning_columns: Vec<String>,
) -> Result<Value, RunnerError> {
    let results = run_mutation(
        pool,
        schema,
        object_name_singular,
        kind,
        filter,
        data,
        actor,
        with_deleted,
        returning_columns,
    )
    .await?;

    results
        .into_iter()
        .next()
        .ok_or_else(RunnerError::record_not_found)
}

#[allow(clippy::too_many_arguments)]
pub async fn run_mutation_one_with_snapshot(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    kind: FilteredMutationKind,
    filter: Option<Value>,
    data: Option<Value>,
    actor: Option<&ActorContext>,
    with_deleted: bool,
    returning_columns: Vec<String>,
) -> Result<MutationSnapshot, RunnerError> {
    run_mutation_with_snapshots(
        pool,
        schema,
        object_name_singular,
        kind,
        filter,
        data,
        actor,
        with_deleted,
        returning_columns,
    )
    .await?
    .into_iter()
    .next()
    .ok_or_else(RunnerError::record_not_found)
}

#[allow(clippy::too_many_arguments)]
pub async fn require_filtered_mutation(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    kind: FilteredMutationKind,
    filter: Option<Value>,
    data: Option<Value>,
    actor: Option<&ActorContext>,
    with_deleted: bool,
    returning_columns: Vec<String>,
) -> Result<usize, RunnerError> {
    if filter
        .as_ref()
        .and_then(Value::as_object)
        .is_none_or(|filter| filter.is_empty())
    {
        return Err(RunnerError::new(
            "INVALID_ARGS_FILTER",
            "A filtered mutation requires a non-empty filter",
        ));
    }

    let results = run_mutation(
        pool,
        schema,
        object_name_singular,
        kind,
        filter,
        data,
        actor,
        with_deleted,
        returning_columns,
    )
    .await?;

    Ok(results.len())
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::metadata::flat::FieldMetadataType;
    use crate::metadata::flat::FlatEntityMaps;
    use crate::metadata::flat::FlatFieldMetadata;
    use crate::metadata::flat::FlatObjectMetadata;
    use crate::query::runner::test_support::DB_TEST_LOCK;
    use serde_json::json;

    #[tokio::test]
    async fn update_rejects_and_rolls_back_rows_that_lose_access() -> Result<(), RunnerError> {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return Ok(());
        };
        let _guard = DB_TEST_LOCK.lock().await;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .map_err(RunnerError::database)?;
        let workspace_id = Uuid::new_v4();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let schema_sql = quote_identifier(&schema_name)?;
        let table_sql = format!("{schema_sql}.{}", quote_identifier("item")?);
        let object_id = Uuid::new_v4();
        let id_field = FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: FieldMetadataType::Uuid,
            name: String::from("id"),
            label: String::from("id"),
            ..Default::default()
        };
        let job_title_field = FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            name: String::from("jobTitle"),
            label: String::from("job title"),
            ..Default::default()
        };
        let mut fields = FlatEntityMaps::default();

        for field in [&id_field, &job_title_field] {
            fields.add(
                field.id.to_string(),
                field.clone(),
                field.id.to_string(),
                workspace_id.to_string(),
            );
        }

        let object = FlatObjectMetadata {
            id: object_id,
            workspace_id,
            name_singular: String::from("item"),
            name_plural: String::from("items"),
            label_singular: String::from("item"),
            label_plural: String::from("items"),
            target_table_name: Some(String::from("item")),
            field_ids: vec![id_field.id.to_string(), job_title_field.id.to_string()],
            ..Default::default()
        };
        let rule = crate::permissions::RowLevelRecordsPermissions {
            rule_id: Uuid::new_v4().to_string(),
            can_update_records: true,
            predicates: vec![json!({
                "id": Uuid::new_v4().to_string(),
                "fieldMetadataId": job_title_field.id.to_string(),
                "objectMetadataId": object_id.to_string(),
                "operand": "IS",
                "value": "Engineer"
            })],
            ..Default::default()
        };
        let mut permissions = crate::permissions::RoleRecordsPermissions {
            objects: [(object_id.to_string(), Default::default())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let object_permission = permissions
            .objects
            .get_mut(&object_id.to_string())
            .expect("object permission");
        object_permission.object_metadata_id = object_id.to_string();
        object_permission.row_level_records_permissions = Some(vec![rule]);
        let schema = RunnerSchema::build_with_permissions(
            &workspace_id,
            &[object],
            &fields,
            &FlatEntityMaps::default(),
            permissions,
            None,
        )?;

        sqlx::query(&format!("CREATE SCHEMA {schema_sql}"))
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;
        sqlx::query(&format!(
            "CREATE TABLE {table_sql} (id uuid PRIMARY KEY, \"jobTitle\" text NOT NULL)"
        ))
        .execute(&pool)
        .await
        .map_err(RunnerError::database)?;
        let record_id = Uuid::new_v4();
        sqlx::query(&format!(
            "INSERT INTO {table_sql} (id, \"jobTitle\") VALUES ($1, 'Engineer')"
        ))
        .bind(record_id)
        .execute(&pool)
        .await
        .map_err(RunnerError::database)?;
        let filter = json!({ "id": { "eq": record_id.to_string() } });
        let rejected = run_mutation_one(
            &pool,
            &schema,
            "item",
            FilteredMutationKind::Update,
            Some(filter.clone()),
            Some(json!({ "jobTitle": "Manager" })),
            None,
            false,
            vec![String::from("jobTitle")],
        )
        .await;
        let allowed = run_mutation_one(
            &pool,
            &schema,
            "item",
            FilteredMutationKind::Update,
            Some(filter),
            Some(json!({ "jobTitle": "Engineer" })),
            None,
            false,
            vec![String::from("jobTitle")],
        )
        .await;
        let stored_title = sqlx::query_scalar::<_, String>(&format!(
            "SELECT \"jobTitle\" FROM {table_sql} WHERE id = $1"
        ))
        .bind(record_id)
        .fetch_one(&pool)
        .await
        .map_err(RunnerError::database)?;
        sqlx::query(&format!("DROP SCHEMA {schema_sql} CASCADE"))
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;

        let rejected = rejected.expect_err("access-losing update must fail");
        assert_eq!(rejected.code(), "RECORD_DOES_NOT_SATISFY_RLS");
        assert_eq!(
            rejected.message(),
            "Updated record does not satisfy row-level security constraints of your current role"
        );
        assert_eq!(stored_title, "Engineer");
        assert_eq!(
            allowed?,
            json!({ "item_jobTitle": "Engineer" }),
            "internal id must not leak into mutation results"
        );
        Ok(())
    }
}
