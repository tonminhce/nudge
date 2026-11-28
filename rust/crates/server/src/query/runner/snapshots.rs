use std::collections::HashMap;

use serde_json::Value;
use sqlx::Executor;
use uuid::Uuid;

use super::base::render_select_list;
use super::base::ObjectModel;
use super::base::RunnerSchema;
use super::format_result_rows;
use super::RunnerError;
use crate::query::repository::quote_identifier;
use crate::query::repository::table_shape::ColumnShape;

pub fn readable_stored_columns(schema: &RunnerSchema, model: &ObjectModel) -> Vec<ColumnShape> {
    model
        .shape
        .columns
        .iter()
        .filter(|column| !schema.is_field_unreadable(model.object.id, column.field_metadata_id))
        .cloned()
        .collect()
}

pub fn stored_column_names(columns: &[ColumnShape]) -> Vec<String> {
    columns
        .iter()
        .map(|column| column.column_name.clone())
        .collect()
}

pub fn snapshot_from_stored_row(
    row: &sqlx::postgres::PgRow,
    model: &ObjectModel,
    columns: &[ColumnShape],
) -> Result<Value, RunnerError> {
    let mut values = serde_json::Map::new();
    for column in columns {
        let alias =
            super::base::column_result_alias(&model.object.name_singular, &column.column_name);
        values.insert(alias.clone(), super::base::json_cell(row, &alias)?);
    }
    let mapped = columns
        .iter()
        .map(|column| {
            let alias =
                super::base::column_result_alias(&model.object.name_singular, &column.column_name);
            (column.clone(), alias.clone(), alias)
        })
        .collect::<Vec<_>>();
    format_result_rows(model, &mapped, &[values])?
        .into_iter()
        .next()
        .ok_or_else(RunnerError::record_not_found)
}

pub async fn read_stored_records<'e, E>(
    executor: E,
    model: &ObjectModel,
    columns: &[ColumnShape],
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Value>, RunnerError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let alias = model.object.name_singular.as_str();
    let selected = columns
        .iter()
        .map(|column| {
            (
                column.clone(),
                super::base::column_result_alias(alias, &column.column_name),
                column.field_name.clone(),
            )
        })
        .collect::<Vec<_>>();
    let sql = format!(
        "SELECT {} FROM {}.{} AS {} WHERE {}.{} = ANY($1::uuid[])",
        render_select_list(alias, &selected, &[])?,
        quote_identifier(&model.shape.schema_name)?,
        quote_identifier(&model.shape.table_name)?,
        quote_identifier(alias)?,
        quote_identifier(alias)?,
        quote_identifier("id")?,
    );
    let rows = sqlx::query(&sql)
        .bind(ids.to_vec())
        .fetch_all(executor)
        .await
        .map_err(RunnerError::database)?;
    let mut records = HashMap::new();
    for row in rows {
        let snapshot = snapshot_from_stored_row(&row, model, columns)?;
        let id = snapshot
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(RunnerError::record_not_found)?;
        records.insert(id, snapshot);
    }
    Ok(records)
}
