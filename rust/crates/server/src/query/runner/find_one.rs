use serde_json::Value;

use crate::query::filter::Suffixes;

use super::base::compile_runner_filter;
use super::base::execute_json_rows;
use super::base::resolve_where_clauses;
use super::base::RunnerSchema;
use super::base::Selection;
use super::find_many::NestedRelationSelection;
use super::format_result_rows;
use super::permissions::validate_filter_read;
use super::permissions::validate_read;
use super::rls::row_access_filter;
use super::RunnerError;

#[derive(Debug, Clone, Default)]
pub struct FindOneArgs {
    pub filter: Option<Value>,
    pub with_deleted: bool,
    pub selected_field_names: Vec<String>,
}

pub async fn find_one(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    args: &FindOneArgs,
    nested: &[NestedRelationSelection],
) -> Result<Option<Value>, RunnerError> {
    let Some(filter) = args.filter.as_ref() else {
        return Err(RunnerError::missing_filter());
    };

    let model = schema.object(object_name_singular)?;
    let alias = object_name_singular;
    validate_filter_read(schema, model, Some(filter))?;
    let mut suffixes = Suffixes::new();
    let compiled_filter =
        compile_runner_filter(schema, object_name_singular, Some(filter), &mut suffixes)?;
    let mut parameter_map = serde_json::Map::new();
    let mut clauses;

    if let Some(compiled_filter) = compiled_filter {
        clauses = compiled_filter.clauses;
        parameter_map.extend(compiled_filter.params);
    } else {
        clauses = Vec::new();
    }
    if let Some(access) = row_access_filter(
        pool,
        schema,
        object_name_singular,
        crate::permissions::ObjectRecordsOperationType::Read,
        &mut suffixes,
    )
    .await?
    {
        clauses.extend(access.clauses);
        parameter_map.extend(access.params);
    }
    let where_expression = resolve_where_clauses(schema, object_name_singular, alias, &clauses)?;
    let mut predicates = Vec::new();

    if !where_expression.is_empty() {
        predicates.push(where_expression);
    }

    if !args.with_deleted && model.shape.has_deleted_at_column {
        predicates.push(format!(
            "{}.{} IS NULL",
            crate::query::repository::quote_identifier(alias)?,
            crate::query::repository::quote_identifier("deletedAt")?
        ));
    }

    let where_sql = if predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", predicates.join(" AND "))
    };
    let columns = schema.select_read_columns(
        model,
        &Selection::from_fields(args.selected_field_names.clone()),
    )?;
    validate_read(schema, model, &columns)?;
    let mapped_columns = columns
        .into_iter()
        .map(|column| {
            let result_alias = super::base::column_result_alias(alias, &column.column_name);
            (column, result_alias.clone(), result_alias)
        })
        .collect::<Vec<_>>();
    let select_sql = super::base::render_select_list(alias, &mapped_columns, &[])?;
    let sql = format!(
        "SELECT {select_sql} FROM {}.{} AS {}{where_sql} ORDER BY {}.{} ASC NULLS LAST LIMIT 2",
        crate::query::repository::quote_identifier(&model.shape.schema_name)?,
        crate::query::repository::quote_identifier(&model.shape.table_name)?,
        crate::query::repository::quote_identifier(alias)?,
        crate::query::repository::quote_identifier(alias)?,
        crate::query::repository::quote_identifier("id")?
    );
    let compiled_sql = crate::query::compile::compile_named_parameters(&sql, &parameter_map)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let raw_rows = execute_json_rows(pool, &compiled_sql.text, &compiled_sql.values).await?;
    let mut records = format_result_rows(model, &mapped_columns, &raw_rows)?;

    if records.len() > 1 {
        return Err(RunnerError::invalid_query(
            "the findOne filter resolved more than one record",
        ));
    }

    if !nested.is_empty() {
        super::nested::load_nested_relations(
            pool,
            schema,
            object_name_singular,
            &mut records,
            nested,
        )
        .await?;
    }

    Ok(records.into_iter().next())
}
