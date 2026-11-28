use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use crate::query::cursor::encode_cursor_data;
use crate::query::repository::table_shape::ColumnShape;

use super::base::lowercase_first;
use super::base::RunnerSchema;
use super::base::QUERY_MAX_RECORDS;
use super::find_many::find_many;
use super::find_many::FindManyArgs;
use super::find_many::NestedRelationSelection;
use super::RunnerError;

pub async fn find_duplicates(
    pool: &sqlx::PgPool,
    schema: Arc<RunnerSchema>,
    object_name_singular: &str,
    ids: &[Uuid],
    data: Option<&[Value]>,
    selected_fields: Vec<String>,
    nested: Vec<NestedRelationSelection>,
) -> Result<Value, RunnerError> {
    if ids.is_empty() && data.is_none() {
        return Ok(Value::Array(Vec::new()));
    }
    let model = schema.object(object_name_singular)?;
    let criteria = duplicate_criteria(model, &schema)?;
    if criteria.is_empty() {
        let count = data.map(|values| values.len()).unwrap_or(ids.len());
        return Ok(Value::Array(
            (0..count).map(|_| empty_connection()).collect(),
        ));
    }
    let mut source_fields = vec![String::from("id")];
    for group in &criteria {
        for column in group {
            if !source_fields.contains(&column.field_name) {
                source_fields.push(column.field_name.clone());
            }
        }
    }
    let sources = if let Some(data) = data {
        data.to_vec()
    } else {
        let source_ids = ids
            .iter()
            .map(|id| Value::String(id.to_string()))
            .collect::<Vec<_>>();
        let mut sources = find_many(
            pool,
            &schema,
            object_name_singular,
            &FindManyArgs {
                filter: Some(json!({"id": {"in": source_ids}})),
                selected_field_names: source_fields,
                skip_channel_visibility: false,
                ..Default::default()
            },
            false,
            &[],
        )
        .await?
        .records;
        let order = ids
            .iter()
            .enumerate()
            .map(|(index, id)| (id.to_string(), index))
            .collect::<HashMap<_, _>>();
        sources.sort_by_key(|record| {
            record
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| order.get(id).copied())
                .unwrap_or(usize::MAX)
        });
        sources
    };

    let mut tasks = tokio::task::JoinSet::new();
    for (index, source) in sources.into_iter().enumerate() {
        let pool = pool.clone();
        let schema = schema.clone();
        let object_name_singular = object_name_singular.to_string();
        let selected_fields = selected_fields.clone();
        let nested = nested.clone();
        let criteria = criteria.clone();
        tasks.spawn(async move {
            (
                index,
                duplicate_connection(
                    &pool,
                    schema,
                    &object_name_singular,
                    &source,
                    &criteria,
                    selected_fields,
                    nested,
                )
                .await,
            )
        });
    }
    let mut connections = vec![None; tasks.len()];
    while let Some(result) = tasks.join_next().await {
        let (index, result) = result.map_err(|error| {
            RunnerError::new("RECORD_PERMISSION_QUERY_INVALID", error.to_string())
        })?;
        connections[index] = Some(result?);
    }
    Ok(Value::Array(connections.into_iter().flatten().collect()))
}

fn duplicate_criteria(
    model: &super::base::ObjectModel,
    schema: &RunnerSchema,
) -> Result<Vec<Vec<ColumnShape>>, RunnerError> {
    let Some(criteria) = model.object.duplicate_criteria.as_ref() else {
        return Ok(Vec::new());
    };
    let mut groups = Vec::new();
    for group in criteria.as_array().into_iter().flatten() {
        let mut columns = Vec::new();
        for name in group.as_array().into_iter().flatten() {
            let Some(name) = name.as_str() else {
                continue;
            };
            let Some(column) = model.shape.column(name).cloned() else {
                return Err(RunnerError::unknown_column(
                    name,
                    &model.object.name_singular,
                ));
            };
            if !schema.is_field_unreadable(model.object.id, column.field_metadata_id) {
                columns.push(column);
            }
        }
        if columns.len() == group.as_array().map(Vec::len).unwrap_or(0) && !columns.is_empty() {
            groups.push(columns);
        }
    }
    Ok(groups)
}

async fn duplicate_connection(
    pool: &sqlx::PgPool,
    schema: Arc<RunnerSchema>,
    object_name_singular: &str,
    source: &Value,
    criteria: &[Vec<ColumnShape>],
    selected_fields: Vec<String>,
    nested: Vec<NestedRelationSelection>,
) -> Result<Value, RunnerError> {
    let Some(filter) = duplicate_filter(source, criteria) else {
        return Ok(empty_connection());
    };
    let result = find_many(
        pool,
        &schema,
        object_name_singular,
        &FindManyArgs {
            filter: Some(filter),
            order_by: Some(json!([{"id": "AscNullsFirst"}])),
            first: Some(QUERY_MAX_RECORDS),
            selected_field_names: selected_fields,
            ..Default::default()
        },
        true,
        &nested,
    )
    .await?;
    let mut edges = Vec::with_capacity(result.records.len());
    for record in result.records {
        let id = record.get("id").and_then(Value::as_str).unwrap_or_default();
        let cursor = result.cursors.get(id).cloned().unwrap_or_else(|| {
            encode_cursor_data(&Map::from_iter([(
                String::from("id"),
                Value::String(id.to_string()),
            )]))
        });
        edges.push(json!({"node": record, "cursor": cursor}));
    }
    let start_cursor = edges
        .first()
        .and_then(|edge| edge.get("cursor"))
        .cloned()
        .unwrap_or(Value::Null);
    let end_cursor = edges
        .last()
        .and_then(|edge| edge.get("cursor"))
        .cloned()
        .unwrap_or(Value::Null);
    Ok(json!({
        "edges": edges,
        "pageInfo": {
            "startCursor": start_cursor,
            "endCursor": end_cursor,
            "hasNextPage": false,
            "hasPreviousPage": false,
        },
        "totalCount": result.total_count.unwrap_or(0),
    }))
}

fn duplicate_filter(source: &Value, criteria: &[Vec<ColumnShape>]) -> Option<Value> {
    let source_id = source.get("id").and_then(Value::as_str);
    let mut conditions = Vec::new();
    for group in criteria {
        let mut values = Vec::with_capacity(group.len());
        for column in group {
            let Some(value) = record_column_value(source, column).and_then(Value::as_str) else {
                values.clear();
                break;
            };
            if value.encode_utf16().count() < 3 {
                values.clear();
                break;
            }
            values.push((column, value));
        }
        if values.len() != group.len() {
            continue;
        }
        let mut condition = Map::new();
        for (column, value) in values {
            if let Some(parent) = column.composite_parent_field_name.as_deref() {
                let sub_field = lowercase_first(
                    column
                        .column_name
                        .strip_prefix(parent)
                        .unwrap_or(&column.column_name),
                );
                let entry = condition
                    .entry(parent.to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                entry
                    .as_object_mut()
                    .expect("composite condition")
                    .insert(sub_field, json!({"eq": value}));
            } else {
                condition.insert(column.field_name.clone(), json!({"eq": value}));
            }
        }
        conditions.push(Value::Object(condition));
    }
    if conditions.is_empty() {
        return None;
    }
    let mut filter = Map::new();
    filter.insert(String::from("or"), Value::Array(conditions));
    if let Some(source_id) = source_id {
        filter.insert(String::from("id"), json!({"neq": source_id}));
    }
    Some(Value::Object(filter))
}

fn record_column_value<'a>(record: &'a Value, column: &ColumnShape) -> Option<&'a Value> {
    if let Some(parent) = column.composite_parent_field_name.as_deref() {
        let sub_field = lowercase_first(
            column
                .column_name
                .strip_prefix(parent)
                .unwrap_or(&column.column_name),
        );
        return record.get(parent)?.get(sub_field);
    }
    record.get(&column.field_name)
}

fn empty_connection() -> Value {
    json!({
        "edges": [],
        "pageInfo": {
            "startCursor": Value::Null,
            "endCursor": Value::Null,
            "hasNextPage": false,
            "hasPreviousPage": false,
        },
        "totalCount": 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::repository::table_shape::ColumnShape;

    fn column(name: &str, parent: Option<&str>) -> ColumnShape {
        ColumnShape {
            column_name: name.to_string(),
            field_metadata_id: Uuid::new_v4(),
            field_name: parent.unwrap_or(name).to_string(),
            field_metadata_type: crate::metadata::flat::FieldMetadataType::Text,
            composite_parent_field_name: parent.map(str::to_string),
            is_array: false,
            empty_string_default: false,
        }
    }

    #[test]
    fn duplicate_filter_expands_composites_and_ignores_short_values() {
        let criteria = vec![vec![
            column("nameFirstName", Some("name")),
            column("nameLastName", Some("name")),
        ]];
        let source = json!({
            "id": "p1",
            "name": {"firstName": "Ada", "lastName": "Lovelace"}
        });
        assert_eq!(
            duplicate_filter(&source, &criteria),
            Some(json!({
                "or": [{"name": {"firstName": {"eq": "Ada"}, "lastName": {"eq": "Lovelace"}}}],
                "id": {"neq": "p1"}
            }))
        );
        let no_id = json!({
            "name": {"firstName": "Ada", "lastName": "Lovelace"}
        });
        assert_eq!(
            duplicate_filter(&no_id, &criteria),
            Some(json!({
                "or": [{"name": {"firstName": {"eq": "Ada"}, "lastName": {"eq": "Lovelace"}}}]
            }))
        );
        let criteria_with_missing_group = vec![
            criteria[0].clone(),
            vec![column("emailsPrimaryEmail", Some("emails"))],
        ];
        assert_eq!(
            duplicate_filter(&no_id, &criteria_with_missing_group),
            Some(json!({
                "or": [{"name": {"firstName": {"eq": "Ada"}, "lastName": {"eq": "Lovelace"}}}]
            }))
        );
        let short = json!({"id": "p1", "name": {"firstName": "Al", "lastName": "X"}});
        assert_eq!(duplicate_filter(&short, &criteria), None);
    }
}
