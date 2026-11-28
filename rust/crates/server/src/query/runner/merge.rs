use std::collections::HashMap;
use std::collections::HashSet;

use chrono::SecondsFormat;
use chrono::Utc;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

use crate::metadata::flat::FieldMetadataType;

use super::base::RunnerSchema;
use super::find_many::find_many;
use super::find_many::FindManyArgs;
use super::find_many::NestedRelationSelection;
use super::RunnerError;

#[derive(Debug, Clone)]
pub struct MergeManyArgs {
    pub ids: Vec<Uuid>,
    pub conflict_priority_index: usize,
    pub dry_run: bool,
}

pub async fn merge_many(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    args: &MergeManyArgs,
    selected_fields: Vec<String>,
    nested: Vec<NestedRelationSelection>,
) -> Result<Value, RunnerError> {
    validate_args(schema, object_name_singular, args)?;
    let model = schema.object(object_name_singular)?;
    if model
        .fields_by_name
        .values()
        .any(|field| !field.is_system && schema.is_field_unreadable(model.object.id, field.id))
    {
        return Err(RunnerError::permission_denied(
            "Merging records with unreadable fields is not allowed",
        ));
    }
    let ids = args
        .ids
        .iter()
        .map(|id| Value::String(id.to_string()))
        .collect::<Vec<_>>();
    let result = find_many(
        pool,
        schema,
        object_name_singular,
        &FindManyArgs {
            filter: Some(json!({"id": {"in": ids}})),
            selected_field_names: readable_field_names(schema, model),
            ..Default::default()
        },
        false,
        &nested,
    )
    .await?;
    let order = args
        .ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.to_string(), index))
        .collect::<HashMap<_, _>>();
    let mut records = result.records;
    records.sort_by_key(|record| {
        record
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| order.get(id).copied())
            .unwrap_or(usize::MAX)
    });
    if records.len() != args.ids.len() {
        return Err(RunnerError::new(
            "RECORD_NOT_FOUND",
            "One or more records to merge were not found",
        ));
    }
    let mut preview = records[args.conflict_priority_index].clone();
    let ordered = ordered_records(&records, args.conflict_priority_index);

    for field in model.fields_by_name.values() {
        if field.is_system {
            continue;
        }
        let values = records
            .iter()
            .map(|record| record.get(&field.name).cloned().unwrap_or(Value::Null))
            .collect::<Vec<_>>();
        let priority_values = ordered
            .iter()
            .map(|record| record.get(&field.name).cloned().unwrap_or(Value::Null))
            .collect::<Vec<_>>();
        let merged = match field.field_type {
            FieldMetadataType::Array | FieldMetadataType::MultiSelect => stable_union(
                values
                    .iter()
                    .filter_map(Value::as_array)
                    .flatten()
                    .filter(|value| has_record_field_value(value))
                    .cloned(),
            ),
            FieldMetadataType::Emails => merge_emails(&values),
            FieldMetadataType::Phones => merge_phones(&values),
            FieldMetadataType::Links => merge_links(&values),
            FieldMetadataType::Relation | FieldMetadataType::MorphRelation => {
                merge_relation_or_scalar(values, priority_values)
            }
            _ => first_meaningful(priority_values),
        };
        preview[&field.name] = merged;
    }
    preview["id"] = Value::String(Uuid::new_v4().to_string());
    preview["deletedAt"] = Value::String(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true));
    let _ = selected_fields;
    Ok(preview)
}

fn validate_args(
    schema: &RunnerSchema,
    object_name_singular: &str,
    args: &MergeManyArgs,
) -> Result<(), RunnerError> {
    if !(2..=9).contains(&args.ids.len()) {
        return Err(RunnerError::invalid_query(
            "You need to provide between 2 and 9 records to merge",
        ));
    }
    let unique = args.ids.iter().copied().collect::<HashSet<_>>();
    if unique.len() != args.ids.len() {
        return Err(RunnerError::invalid_query("Record ids must be unique"));
    }
    if args.conflict_priority_index >= args.ids.len() {
        return Err(RunnerError::invalid_query(
            "conflictPriorityIndex must reference one of the provided records",
        ));
    }
    let object = schema.object(object_name_singular)?;
    if object.object.is_remote || object.object.is_system {
        return Err(RunnerError::invalid_query(
            "Records from remote or system objects cannot be merged",
        ));
    }
    if !args.dry_run {
        return Err(RunnerError::invalid_query(
            "Only merge dry runs are currently available",
        ));
    }
    Ok(())
}

fn readable_field_names(schema: &RunnerSchema, model: &super::base::ObjectModel) -> Vec<String> {
    let mut names = Vec::new();
    for column in &model.shape.columns {
        if schema.is_field_unreadable(model.object.id, column.field_metadata_id) {
            continue;
        }
        if !names.contains(&column.field_name) {
            names.push(column.field_name.clone());
        }
    }
    if !names.iter().any(|name| name == "id") {
        names.insert(0, String::from("id"));
    }
    names
}

fn ordered_records(records: &[Value], priority: usize) -> Vec<&Value> {
    let mut ordered = vec![&records[priority]];
    ordered.extend(
        records
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != priority)
            .map(|(_, record)| record),
    );
    ordered
}

fn first_meaningful(values: Vec<Value>) -> Value {
    let fallback = values.first().cloned().unwrap_or(Value::Null);
    values
        .into_iter()
        .find(has_record_field_value)
        .unwrap_or(fallback)
}

fn merge_relation_or_scalar(values: Vec<Value>, priority_values: Vec<Value>) -> Value {
    if let Some(Value::Array(items)) = values.iter().find(|value| value.is_array()) {
        return stable_union(
            items
                .iter()
                .filter(|value| has_record_field_value(value))
                .cloned(),
        );
    }
    first_meaningful(priority_values)
}

fn stable_union(values: impl IntoIterator<Item = Value>) -> Value {
    let mut output = Vec::new();
    for value in values {
        if !output.contains(&value) {
            output.push(value);
        }
    }
    if output.is_empty() {
        Value::Array(Vec::new())
    } else {
        Value::Array(output)
    }
}

fn merge_emails(records: &[Value]) -> Value {
    let primary = records
        .iter()
        .find_map(|record| meaningful_string(record.get("primaryEmail")))
        .unwrap_or_default();
    let mut emails = Vec::new();
    for record in records {
        if let Some(value) = meaningful_string(record.get("primaryEmail")) {
            push_unique(&mut emails, value);
        }
        if let Some(values) = record.get("additionalEmails").and_then(Value::as_array) {
            for value in values.iter().filter_map(Value::as_str) {
                push_unique(&mut emails, value.to_string());
            }
        }
    }
    emails.retain(|email| email != &primary);
    json!({
        "primaryEmail": primary,
        "additionalEmails": if emails.is_empty() { json!([]) } else { json!(emails) },
    })
}

fn merge_phones(records: &[Value]) -> Value {
    let priority = records
        .iter()
        .find(|record| meaningful_string(record.get("primaryPhoneNumber")).is_some())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut additional = Vec::new();
    for record in records {
        if let Some(number) = meaningful_string(record.get("primaryPhoneNumber")) {
            push_phone_unique(&mut additional, record, number);
        }
        if let Some(values) = record.get("additionalPhones").and_then(Value::as_array) {
            for value in values {
                if let Some(number) = value
                    .get("number")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                {
                    push_phone_unique(&mut additional, value, number.to_string());
                }
            }
        }
    }
    if let Some(primary) = meaningful_string(priority.get("primaryPhoneNumber")) {
        additional.retain(|phone| phone["number"].as_str() != Some(&primary));
    }
    json!({
        "primaryPhoneNumber": priority.get("primaryPhoneNumber").cloned().unwrap_or_else(|| Value::String(String::new())),
        "primaryPhoneCountryCode": priority.get("primaryPhoneCountryCode").cloned().unwrap_or_else(|| Value::String(String::new())),
        "primaryPhoneCallingCode": priority.get("primaryPhoneCallingCode").cloned().unwrap_or_else(|| Value::String(String::new())),
        "additionalPhones": if additional.is_empty() { json!([]) } else { json!(additional) },
    })
}

fn push_phone_unique(output: &mut Vec<Value>, record: &Value, number: String) {
    if output
        .iter()
        .any(|value| value["number"].as_str() == Some(number.as_str()))
    {
        return;
    }
    output.push(json!({
        "number": number,
        "countryCode": record.get("countryCode").or_else(|| record.get("primaryPhoneCountryCode")).cloned().unwrap_or(Value::Null),
        "callingCode": record.get("callingCode").or_else(|| record.get("primaryPhoneCallingCode")).cloned().unwrap_or(Value::Null),
    }));
}

fn merge_links(records: &[Value]) -> Value {
    let priority = records
        .iter()
        .find(|record| meaningful_string(record.get("primaryLinkUrl")).is_some())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut secondary = Vec::new();
    for record in records {
        if let Some(url) = meaningful_string(record.get("primaryLinkUrl")) {
            push_link_unique(
                &mut secondary,
                record
                    .get("primaryLinkLabel")
                    .cloned()
                    .unwrap_or(Value::Null),
                url,
            );
        }
        if let Some(values) = record.get("secondaryLinks").and_then(Value::as_array) {
            for value in values {
                if let Some(url) = value.get("url").and_then(Value::as_str) {
                    push_link_unique(
                        &mut secondary,
                        value.get("label").cloned().unwrap_or(Value::Null),
                        url.to_string(),
                    );
                }
            }
        }
    }
    if let Some(primary) = meaningful_string(priority.get("primaryLinkUrl")) {
        secondary.retain(|link| link["url"].as_str() != Some(&primary));
    }
    json!({
        "primaryLinkLabel": priority.get("primaryLinkLabel").cloned().unwrap_or_else(|| Value::String(String::new())),
        "primaryLinkUrl": priority.get("primaryLinkUrl").cloned().unwrap_or_else(|| Value::String(String::new())),
        "secondaryLinks": if secondary.is_empty() { json!([]) } else { json!(secondary) },
    })
}

fn push_link_unique(output: &mut Vec<Value>, label: Value, url: String) {
    if output
        .iter()
        .any(|value| value["url"].as_str() == Some(url.as_str()))
    {
        return;
    }
    output.push(json!({"url": url, "label": label}));
}

fn meaningful_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn push_unique(output: &mut Vec<String>, value: String) {
    if !output.contains(&value) {
        output.push(value);
    }
}

fn has_record_field_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
        Value::Bool(_) => true,
        Value::Number(value) => value.as_f64().is_some_and(|value| !value.is_nan()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn composite_merges_keep_priority_primaries_and_union_extras() {
        let records = vec![
            json!({
                "primaryEmail": "first@example.com",
                "additionalEmails": ["shared@example.com"],
                "primaryPhoneNumber": "111",
                "primaryPhoneCountryCode": "US",
                "primaryPhoneCallingCode": "+1",
                "additionalPhones": [{"number": "222", "countryCode": "US", "callingCode": "+1"}],
                "primaryLinkUrl": "https://first.example.com",
                "primaryLinkLabel": "First",
                "secondaryLinks": [{"url": "https://shared.example.com", "label": "Shared"}]
            }),
            json!({
                "primaryEmail": "second@example.com",
                "additionalEmails": ["shared@example.com", "third@example.com"],
                "primaryPhoneNumber": "333",
                "primaryPhoneCountryCode": "GB",
                "primaryPhoneCallingCode": "+44",
                "additionalPhones": [{"number": "222", "countryCode": "GB", "callingCode": "+44"}],
                "primaryLinkUrl": "https://second.example.com",
                "primaryLinkLabel": "Second",
                "secondaryLinks": [{"url": "https://shared.example.com", "label": "Other"}]
            }),
        ];

        assert_eq!(
            merge_emails(&records),
            json!({"primaryEmail": "first@example.com", "additionalEmails": ["shared@example.com", "second@example.com", "third@example.com"]})
        );
        assert_eq!(
            merge_phones(&records),
            json!({
                "primaryPhoneNumber": "111",
                "primaryPhoneCountryCode": "US",
                "primaryPhoneCallingCode": "+1",
                "additionalPhones": [
                    {"number": "222", "countryCode": "US", "callingCode": "+1"},
                    {"number": "333", "countryCode": "GB", "callingCode": "+44"}
                ]
            })
        );
        assert_eq!(
            merge_links(&records),
            json!({
                "primaryLinkLabel": "First",
                "primaryLinkUrl": "https://first.example.com",
                "secondaryLinks": [
                    {"url": "https://shared.example.com", "label": "Shared"},
                    {"url": "https://second.example.com", "label": "Second"}
                ]
            })
        );
    }

    #[test]
    fn scalar_merge_preserves_zero_and_false() {
        assert_eq!(first_meaningful(vec![json!(0), json!(2)]), json!(0));
        assert_eq!(
            first_meaningful(vec![json!(false), json!(true)]),
            json!(false)
        );
        assert_eq!(
            first_meaningful(vec![json!(""), json!("fallback")]),
            json!("fallback")
        );
    }
}
