use std::collections::HashMap;

use serde_json::Value;

use crate::query::compile::compile_named_parameters;
use crate::query::repository::quote_identifier;

use super::base::column_result_alias;
use super::base::execute_json_rows;
use super::base::render_select_list;
use super::base::select_columns;
use super::base::RunnerSchema;
use super::base::Selection;
use super::base::QUERY_MAX_RECORDS_FROM_RELATION;
use super::find_many::NestedRelationSelection;
use super::format_result_rows;
use super::RunnerError;

pub async fn load_nested_relations(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    records: &mut [Value],
    nested: &[NestedRelationSelection],
) -> Result<(), RunnerError> {
    for selection in nested {
        let (_, relation, child) =
            schema.relation(object_name_singular, &selection.relation_field_name)?;
        let child_alias = format!("{}_{}", object_name_singular, selection.relation_field_name);
        let child_selection = Selection::from_fields(selection.selected_field_names.clone());
        let columns = select_columns(child, &child_selection)?;
        let mapped_columns = columns
            .into_iter()
            .map(|column| {
                let result_alias = column_result_alias(&child_alias, &column.column_name);
                (column, result_alias.clone(), result_alias)
            })
            .collect::<Vec<_>>();
        let select_sql = render_select_list(&child_alias, &mapped_columns, &[])?;

        if let Some(parent_join_column) = relation.join_column_name.as_deref() {
            let mut parent_ids = Vec::new();

            for record in records.iter() {
                let id = record
                    .get(parent_join_column)
                    .or_else(|| record.get(&selection.relation_field_name))
                    .and_then(Value::as_str);

                if let Some(id) = id {
                    parent_ids.push(Value::String(id.to_string()));
                }
            }

            if parent_ids.is_empty() {
                continue;
            }

            let parameters = serde_json::Map::new();
            let mut map = parameters;
            map.insert(String::from("ids"), Value::Array(parent_ids));
            let where_sql = format!(
                "WHERE {}.id = ANY(ARRAY[:...ids]::uuid[])",
                quote_identifier(&child_alias)?
            );
            let sql = format!(
                "SELECT {select_sql} FROM {}.{} AS {} {where_sql} ORDER BY {}.id ASC NULLS LAST",
                quote_identifier(&child.shape.schema_name)?,
                quote_identifier(&child.shape.table_name)?,
                quote_identifier(&child_alias)?,
                quote_identifier(&child_alias)?
            );
            let compiled = compile_named_parameters(&sql, &map)
                .map_err(|error| RunnerError::new(error.code(), error.message()))?;
            let rows = execute_json_rows(pool, &compiled.text, &compiled.values).await?;
            let formatted = format_result_rows(child, &mapped_columns, &rows)?;
            let mut by_id = HashMap::new();

            for record in formatted {
                let id = record.get("id").and_then(Value::as_str).map(str::to_string);

                if let Some(id) = id {
                    by_id.insert(id, record);
                }
            }

            for record in records.iter_mut() {
                let id = record
                    .get(parent_join_column)
                    .or_else(|| record.get(&selection.relation_field_name))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let related = id.and_then(|id| by_id.get(&id).cloned());
                insert_record_field(
                    record,
                    &selection.relation_field_name,
                    related.unwrap_or(Value::Null),
                );
            }

            continue;
        }

        let child_join_column =
            schema.child_join_column(object_name_singular, &selection.relation_field_name)?;
        let mut parent_ids = Vec::new();

        for record in records.iter() {
            if let Some(id) = record.get("id").and_then(Value::as_str) {
                parent_ids.push(Value::String(id.to_string()));
            }
        }

        if parent_ids.is_empty() {
            continue;
        }

        let mut map = serde_json::Map::new();
        map.insert(String::from("ids"), Value::Array(parent_ids));
        let live_predicate = if child.shape.has_deleted_at_column {
            format!(
                "{}.{} IS NULL",
                quote_identifier(&child_alias)?,
                quote_identifier("deletedAt")?
            )
        } else {
            String::from("TRUE")
        };
        let where_sql = format!(
            "WHERE {}.{} = ANY(ARRAY[:...ids]::uuid[]) AND {live_predicate}",
            quote_identifier(&child_alias)?,
            quote_identifier(&child_join_column)?
        );
        let sql = format!(
            "SELECT {select_sql} FROM {}.{} AS {} {where_sql} ORDER BY {}.id ASC NULLS LAST",
            quote_identifier(&child.shape.schema_name)?,
            quote_identifier(&child.shape.table_name)?,
            quote_identifier(&child_alias)?,
            quote_identifier(&child_alias)?
        );
        let per_parent = selection
            .first
            .unwrap_or(QUERY_MAX_RECORDS_FROM_RELATION)
            .min(QUERY_MAX_RECORDS_FROM_RELATION);
        let hard_limit = per_parent.saturating_mul(records.len().max(1));
        let sql = format!("{sql} LIMIT {hard_limit}");
        let compiled = compile_named_parameters(&sql, &map)
            .map_err(|error| RunnerError::new(error.code(), error.message()))?;
        let rows = execute_json_rows(pool, &compiled.text, &compiled.values).await?;
        let formatted = format_result_rows(child, &mapped_columns, &rows)?;
        let mut grouped: HashMap<String, Vec<Value>> = HashMap::new();

        for (raw, record) in rows.into_iter().zip(formatted) {
            let raw_join_alias = column_result_alias(&child_alias, &child_join_column);
            let parent_id = raw
                .get(&raw_join_alias)
                .and_then(Value::as_str)
                .map(str::to_string);

            if let Some(parent_id) = parent_id {
                let bucket = grouped.entry(parent_id).or_default();

                if bucket.len() < per_parent {
                    bucket.push(record);
                }
            }
        }

        for record in records.iter_mut() {
            let id = record
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_default();
            let related = grouped.remove(&id).unwrap_or_default();
            insert_record_field(
                record,
                &selection.relation_field_name,
                Value::Array(related),
            );
        }
    }

    Ok(())
}

fn insert_record_field(record: &mut Value, field_name: &str, value: Value) {
    if let Some(object) = record.as_object_mut() {
        object.insert(field_name.to_string(), value);
    }
}
