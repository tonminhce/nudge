use serde_json::Map;
use serde_json::Value;
use sqlx::Column;
use sqlx::Row;

use crate::query::filter::Suffixes;
use crate::query::repository::build_mutation_statement;
use crate::query::repository::MutationKind;
use crate::query::repository::MutationStatementState;
use crate::query::repository::MutationWhereClause;

use super::actors::inject_updated_by_on_update;
use super::actors::ActorContext;
use super::base::compile_runner_filter;
use super::base::resolve_where_clauses;
use super::base::RunnerSchema;
use super::permissions::validate_filter_read;
use super::permissions::validate_write;
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

        vec![MutationWhereClause::and(user_where)]
    };
    let mut set_columns = Map::new();

    if let Some(data) = data.as_ref().and_then(Value::as_object) {
        set_columns = data.clone();
    }

    if kind == FilteredMutationKind::Update {
        inject_updated_by_on_update(model, &mut set_columns, actor)?;
    }

    let expanded_columns = if kind == FilteredMutationKind::Update {
        crate::query::repository::expand_record_columns(&model.shape, &set_columns)
            .map_err(RunnerError::from_repository)?
    } else {
        Vec::new()
    };
    let set_columns = expanded_columns.clone();
    let returning_columns = if returning_columns.is_empty() {
        vec![String::from("id")]
    } else {
        returning_columns
    };
    let data = set_columns.iter().cloned().collect::<Map<String, Value>>();
    validate_write(
        schema,
        model,
        kind.permission_operation(),
        &data,
        &returning_columns,
    )?;
    let state = MutationStatementState {
        alias,
        shape: &model.shape,
        kind: kind.repository_kind(),
        set_columns: set_columns.into_iter().collect(),
        where_clauses,
        where_parameters: parameter_map.clone(),
        row_access_conditions: Vec::new(),
        include_deleted: with_deleted,
        returning_columns,
    };
    let statement = build_mutation_statement(&state).map_err(RunnerError::from_repository)?;
    let mut statement_sql = statement.sql;

    for (index, (column_name, _)) in expanded_columns.iter().enumerate() {
        let column = model
            .shape
            .column(column_name)
            .ok_or_else(|| RunnerError::unknown_column(column_name, &model.object.name_singular))?;
        let placeholder = format!(":ormSet_{index}");
        let cast = match column.field_metadata_type {
            crate::metadata::flat::FieldMetadataType::Uuid
            | crate::metadata::flat::FieldMetadataType::Relation
            | crate::metadata::flat::FieldMetadataType::MorphRelation => {
                format!("{placeholder}::uuid")
            }
            crate::metadata::flat::FieldMetadataType::RawJson => format!("{placeholder}::jsonb"),
            crate::metadata::flat::FieldMetadataType::DateTime => {
                format!("{placeholder}::timestamptz")
            }
            crate::metadata::flat::FieldMetadataType::Date => format!("{placeholder}::date"),
            crate::metadata::flat::FieldMetadataType::Select
                if matches!(column_name.as_str(), "createdBySource" | "updatedBySource") =>
            {
                let enum_name = crate::metadata::names::compute_postgres_enum_name(
                    &model.shape.table_name,
                    column_name,
                );
                format!(
                    "{placeholder}::text::{}.{}",
                    crate::query::repository::quote_identifier(&model.shape.schema_name)?,
                    crate::query::repository::quote_identifier(&enum_name)?
                )
            }
            _ => continue,
        };
        statement_sql = statement_sql.replace(&placeholder, &cast);
    }

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

    let rows = query.fetch_all(pool).await.map_err(RunnerError::database)?;
    let mut results = Vec::with_capacity(rows.len());

    for row in rows {
        let mut record = Map::new();

        for column in row.columns() {
            let value = super::base::json_cell(&row, column.name())?;
            record.insert(column.name().to_string(), value);
        }

        results.push(Value::Object(record));
    }

    Ok(results)
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
