use serde_json::Map;
use serde_json::Value;

use crate::metadata::flat::enums::FieldMetadataType;
use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::names::compute_relation_join_column_name;
use crate::query::runner::base::ObjectModel;
use crate::validation::error::ValidationError;
use crate::validation::validate_actor_field;
use crate::validation::validate_address_field;
use crate::validation::validate_array_field;
use crate::validation::validate_boolean_field;
use crate::validation::validate_currency_field;
use crate::validation::validate_date_field;
use crate::validation::validate_date_time_field;
use crate::validation::validate_emails_field;
use crate::validation::validate_files_field;
use crate::validation::validate_full_name_field;
use crate::validation::validate_links_field;
use crate::validation::validate_multi_select_field;
use crate::validation::validate_number_field;
use crate::validation::validate_numeric_field;
use crate::validation::validate_overridden_position_field;
use crate::validation::validate_phones_field;
use crate::validation::validate_rating_and_select_field;
use crate::validation::validate_raw_json_field;
use crate::validation::validate_rich_text_field;
use crate::validation::validate_text_field;
use crate::validation::validate_uuid_field;
use crate::validation::values::is_null;
use crate::validation::ValidationResult;

const RELATION_NESTED_OPERATIONS: [&str; 3] = ["connect", "create", "disconnect"];

fn is_relation_nested_operation(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    RELATION_NESTED_OPERATIONS.iter().any(|operation| {
        object
            .get(*operation)
            .map(|nested| !nested.is_null())
            .unwrap_or(false)
    })
}

fn option_values(field: &FlatFieldMetadata) -> Option<Vec<String>> {
    let options = field.options.as_ref()?;

    if let Some(array) = options.as_array() {
        return Some(
            array
                .iter()
                .filter_map(|option| option.get("value").and_then(Value::as_str))
                .map(str::to_string)
                .collect(),
        );
    }

    if options.is_object() {
        return Some(
            options
                .as_object()
                .map(|entries| {
                    entries
                        .values()
                        .filter_map(|option| option.get("value").and_then(Value::as_str))
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        );
    }

    None
}

fn join_column_name(model: &ObjectModel, field_name: &str) -> String {
    model
        .shape
        .relation(field_name)
        .and_then(|relation| relation.join_column_name.clone())
        .unwrap_or_else(|| compute_relation_join_column_name(field_name))
}

fn field_for_key<'model>(
    model: &'model ObjectModel,
    key: &str,
) -> Option<&'model FlatFieldMetadata> {
    if let Some(field) = model.fields_by_name.get(key) {
        return Some(field);
    }

    model
        .fields_by_name
        .values()
        .filter(|field| {
            matches!(
                field.field_type,
                FieldMetadataType::Relation | FieldMetadataType::MorphRelation
            )
        })
        .find(|field| join_column_name(model, &field.name) == key)
}

fn setting<'field>(field: &'field FlatFieldMetadata, key: &str) -> Option<&'field Value> {
    field
        .settings
        .as_ref()
        .and_then(|settings| settings.get(key))
}

fn validate_field(
    model: &ObjectModel,
    field: &FlatFieldMetadata,
    key: &str,
    value: &Value,
) -> ValidationResult<()> {
    match field.field_type {
        FieldMetadataType::Position => validate_overridden_position_field(value, key),
        FieldMetadataType::Numeric => validate_numeric_field(value, key),
        FieldMetadataType::Number => validate_number_field(value, key),
        FieldMetadataType::Text => validate_text_field(value, key),
        FieldMetadataType::DateTime => validate_date_time_field(value, key),
        FieldMetadataType::Date => validate_date_field(value, key),
        FieldMetadataType::Boolean => validate_boolean_field(value, key),
        FieldMetadataType::Rating | FieldMetadataType::Select => {
            validate_rating_and_select_field(value, key, option_values(field).as_deref())
        }
        FieldMetadataType::MultiSelect => {
            validate_multi_select_field(value, key, option_values(field).as_deref())
        }
        FieldMetadataType::Uuid => validate_uuid_field(value, key),
        FieldMetadataType::Array => validate_array_field(value, key),
        FieldMetadataType::RawJson => validate_raw_json_field(value, key),
        FieldMetadataType::Relation | FieldMetadataType::MorphRelation => {
            validate_relation_field(model, field, key, value)
        }
        FieldMetadataType::Phones => validate_phones_field(value, key),
        FieldMetadataType::Emails => validate_emails_field(value, key),
        FieldMetadataType::Files => validate_files_field(
            value,
            key,
            setting(field, "maxNumberOfValues").and_then(Value::as_u64),
        ),
        FieldMetadataType::FullName => validate_full_name_field(value, key),
        FieldMetadataType::Address => validate_address_field(value, key),
        FieldMetadataType::Currency => validate_currency_field(value, key),
        FieldMetadataType::Actor => validate_actor_field(value, key),
        FieldMetadataType::RichText => validate_rich_text_field(value, key),
        FieldMetadataType::Links => {
            validate_links_field(value, key, setting(field, "type").and_then(Value::as_str))
        }
        FieldMetadataType::TsVector => Err(ValidationError::standard(format!(
            "{key} {}-typed field does not support write operations",
            field.field_type.as_str()
        ))),
    }
}

fn validate_relation_field(
    model: &ObjectModel,
    field: &FlatFieldMetadata,
    key: &str,
    value: &Value,
) -> ValidationResult<()> {
    let one_to_many = setting(field, "relationType").and_then(Value::as_str) == Some("ONE_TO_MANY");

    if one_to_many {
        return Err(ValidationError::standard(format!(
            "One-to-many relation {key} field does not support write operations."
        )));
    }

    if join_column_name(model, &field.name) == key {
        return validate_uuid_field(value, key);
    }

    if !is_relation_nested_operation(value) {
        return Err(ValidationError::standard(format!(
            "Relation \"{key}\" requires create, connect, or disconnect operation"
        )));
    }

    Ok(())
}

fn is_missing_required_value(field: &FlatFieldMetadata, value: &Value) -> bool {
    field.default_value.is_none() && !field.is_nullable && is_null(value)
}

pub fn validate_record_data(
    model: &ObjectModel,
    record: &Map<String, Value>,
) -> ValidationResult<()> {
    for (key, value) in record {
        let Some(field) = field_for_key(model, key) else {
            return Err(ValidationError::standard(format!(
                "Object {} doesn't have any \"{key}\" field.",
                model.object.name_singular
            )));
        };

        if is_missing_required_value(field, value) {
            return Err(ValidationError::new(
                format!("Field {key} is not nullable and has no default value."),
                String::from("A required field is missing."),
            ));
        }

        validate_field(model, field, key, value)?;
    }

    Ok(())
}

pub fn validate_records_data(
    model: &ObjectModel,
    records: &[Map<String, Value>],
) -> ValidationResult<()> {
    for record in records {
        validate_record_data(model, record)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::metadata::flat::FlatEntityMaps;
    use crate::metadata::flat::FlatObjectMetadata;
    use crate::query::repository::table_shape::build_workspace_table_shape;

    fn field(
        id: usize,
        name: &str,
        field_type: FieldMetadataType,
        extra: Option<Value>,
    ) -> FlatFieldMetadata {
        let uuid = Uuid::from_u128(id as u128 + 1);
        let mut settings = match extra {
            Some(value) => value,
            None => json!({}),
        };

        if field_type == FieldMetadataType::Relation {
            settings
                .as_object_mut()
                .unwrap()
                .entry("relationType")
                .or_insert(json!("MANY_TO_ONE"));
        }

        FlatFieldMetadata {
            id: uuid,
            universal_identifier: uuid.to_string(),
            object_metadata_id: Uuid::nil(),
            object_metadata_universal_identifier: "object-id".to_string(),
            field_type,
            name: name.to_string(),
            label: name.to_string(),
            settings: Some(settings),
            is_active: true,
            ..Default::default()
        }
    }

    fn model_with(fields: Vec<FlatFieldMetadata>) -> ObjectModel {
        let mut maps = FlatEntityMaps::default();
        let mut field_ids = Vec::new();
        let mut fields_by_name = HashMap::new();

        for field in fields {
            field_ids.push(field.id.to_string());
            let universal_identifier = field.universal_identifier.clone();
            fields_by_name.insert(field.name.clone(), field.clone());
            maps.add(
                field.id.to_string(),
                field,
                universal_identifier,
                crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER,
            );
        }

        let object = FlatObjectMetadata {
            id: Uuid::nil(),
            universal_identifier: "object-id".to_string(),
            name_singular: "person".to_string(),
            name_plural: "people".to_string(),
            label_singular: "Person".to_string(),
            label_plural: "People".to_string(),
            application_universal_identifier:
                crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER.to_string(),
            target_table_name: Some("person".to_string()),
            field_ids,
            is_active: true,
            ..Default::default()
        };

        ObjectModel {
            shape: Arc::new(build_workspace_table_shape(&Uuid::nil(), &object, &maps)),
            object,
            fields_by_name,
            unique_field_groups: Vec::new(),
        }
    }

    fn record(pairs: Vec<(&str, Value)>) -> Map<String, Value> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    fn error_message(result: ValidationResult<()>) -> String {
        result.err().map(|error| error.message).unwrap_or_default()
    }

    #[test]
    fn unknown_key_reports_the_reference_message() {
        let model = model_with(vec![field(0, "name", FieldMetadataType::Text, None)]);
        let error = validate_record_data(&model, &record(vec![("nope", json!("x"))])).unwrap_err();

        assert_eq!(
            error.message,
            "Object person doesn't have any \"nope\" field."
        );
        assert_eq!(error.code, crate::validation::error::INVALID_ARGS_DATA);
    }

    #[test]
    fn non_nullable_field_without_default_rejects_null() {
        let mut required = field(0, "name", FieldMetadataType::Text, None);
        required.is_nullable = false;
        required.default_value = None;

        let model = model_with(vec![required]);
        let error = validate_record_data(&model, &record(vec![("name", Value::Null)])).unwrap_err();

        assert_eq!(
            error.message,
            "Field name is not nullable and has no default value."
        );
        assert_eq!(
            error.user_friendly_message.as_str(),
            "A required field is missing."
        );
    }

    #[test]
    fn nullable_field_accepts_null() {
        let model = model_with(vec![field(0, "name", FieldMetadataType::Text, None)]);

        assert!(validate_record_data(&model, &record(vec![("name", Value::Null)])).is_ok());
    }

    #[test]
    fn field_with_default_accepts_null() {
        let mut required = field(0, "name", FieldMetadataType::Text, None);
        required.is_nullable = false;
        required.default_value = Some(json!("unnamed"));

        let model = model_with(vec![required]);

        assert!(validate_record_data(&model, &record(vec![("name", Value::Null)])).is_ok());
    }

    #[test]
    fn select_option_violation_surfaces_through_the_record_dispatcher() {
        let mut select = field(0, "stage", FieldMetadataType::Select, None);
        select.options = Some(json!([{ "value": "NEW" }, { "value": "HOT" }]));

        let model = model_with(vec![select]);
        let error =
            validate_record_data(&model, &record(vec![("stage", json!("COLD"))])).unwrap_err();

        assert_eq!(
            error.message,
            "Invalid value \"COLD\" for field \"stage\". Valid values are: NEW, HOT"
        );
    }

    #[test]
    fn missing_options_rejects_every_value() {
        let select = field(0, "stage", FieldMetadataType::Select, None);
        let model = model_with(vec![select]);
        let error = validate_record_data(&model, &record(vec![("stage", json!("x"))])).unwrap_err();

        assert_eq!(error.message, "Invalid options for field \"stage\"");
    }

    #[test]
    fn relation_join_column_validates_as_uuid() {
        let model = model_with(vec![
            field(0, "name", FieldMetadataType::Text, None),
            field(1, "company", FieldMetadataType::Relation, None),
        ]);

        assert!(
            validate_record_data(&model, &record(vec![("companyId", json!("not-a-uuid"))]))
                .is_err()
        );
        assert_eq!(
            error_message(validate_record_data(
                &model,
                &record(vec![(
                    "companyId",
                    json!("3f2504e0-4f89-41d3-9a0c-0305e82c3301")
                )])
            )),
            String::new()
        );
    }

    #[test]
    fn relation_field_requires_a_nested_operation() {
        let model = model_with(vec![field(0, "company", FieldMetadataType::Relation, None)]);
        let error =
            validate_record_data(&model, &record(vec![("company", json!("nope"))])).unwrap_err();

        assert_eq!(
            error.message,
            "Relation \"company\" requires create, connect, or disconnect operation"
        );
        assert!(validate_record_data(
            &model,
            &record(vec![("company", json!({ "connect": { "id": "x" } }))])
        )
        .is_ok());
    }

    #[test]
    fn one_to_many_relation_rejects_writes() {
        let model = model_with(vec![field(
            0,
            "people",
            FieldMetadataType::Relation,
            Some(json!({ "relationType": "ONE_TO_MANY" })),
        )]);
        let error =
            validate_record_data(&model, &record(vec![("people", json!({ "create": [] }))]))
                .unwrap_err();

        assert_eq!(
            error.message,
            "One-to-many relation people field does not support write operations."
        );
    }

    #[test]
    fn ts_vector_rejects_writes() {
        let model = model_with(vec![field(
            0,
            "searchVector",
            FieldMetadataType::TsVector,
            None,
        )]);
        let error = validate_record_data(&model, &record(vec![("searchVector", json!("'a'"))]))
            .unwrap_err();

        assert_eq!(
            error.message,
            "searchVector TS_VECTOR-typed field does not support write operations"
        );
    }

    #[test]
    fn dispatcher_routes_each_field_kind_to_its_validator() {
        let model = model_with(vec![
            field(0, "jobTitle", FieldMetadataType::Text, None),
            field(1, "yearOfBirth", FieldMetadataType::Number, None),
            field(2, "phoneNumbers", FieldMetadataType::Phones, None),
            field(3, "emailAddresses", FieldMetadataType::Emails, None),
            field(4, "primaryEmail", FieldMetadataType::Text, None),
        ]);

        assert_eq!(
            error_message(validate_record_data(
                &model,
                &record(vec![("jobTitle", json!(7))])
            )),
            "Invalid string value 7 for text field \"jobTitle\""
        );
        assert_eq!(
            error_message(validate_record_data(
                &model,
                &record(vec![("yearOfBirth", json!("x"))])
            )),
            "Invalid number value 'x' for field \"yearOfBirth\""
        );
        assert_eq!(
            error_message(validate_record_data(
                &model,
                &record(vec![("phoneNumbers", json!({ "nope": 1 }))])
            )),
            "Invalid subfield nope for phones field \"phoneNumbers\""
        );
        assert_eq!(
            error_message(validate_record_data(
                &model,
                &record(vec![(
                    "emailAddresses",
                    json!({ "primaryEmail": "not-an-email" })
                )])
            )),
            "Invalid string value 'not-an-email' for email field \"emailAddresses.primaryEmail\""
        );
    }

    #[test]
    fn records_data_validates_every_row() {
        let model = model_with(vec![field(0, "name", FieldMetadataType::Text, None)]);

        assert!(validate_records_data(
            &model,
            &[
                record(vec![("name", json!("ada"))]),
                record(vec![("name", json!("grace"))])
            ]
        )
        .is_ok());
        assert!(validate_records_data(
            &model,
            &[
                record(vec![("name", json!("ada"))]),
                record(vec![("name", json!(1))])
            ]
        )
        .is_err());
    }
}
