#![allow(dead_code)]

use serde_json::{Map, Value};

use super::FieldMetadataType;

pub const FIELD_ACTOR_SOURCE_OPTIONS: [&str; 10] = [
    "EMAIL",
    "CALENDAR",
    "WORKFLOW",
    "AGENT",
    "API",
    "IMPORT",
    "MANUAL",
    "SYSTEM",
    "WEBHOOK",
    "APPLICATION",
];

pub struct CompositeProperty {
    pub name: &'static str,
    pub field_type: FieldMetadataType,
    pub is_included_in_unique_constraint: bool,
    pub is_array: bool,
    pub options: &'static [&'static str],
}

pub const COMPOSITE_TYPES: [FieldMetadataType; 8] = [
    FieldMetadataType::Links,
    FieldMetadataType::Currency,
    FieldMetadataType::FullName,
    FieldMetadataType::Address,
    FieldMetadataType::Actor,
    FieldMetadataType::Emails,
    FieldMetadataType::Phones,
    FieldMetadataType::RichText,
];

pub fn is_composite_field_metadata_type(field_type: FieldMetadataType) -> bool {
    COMPOSITE_TYPES.contains(&field_type)
}

pub fn composite_properties(field_type: FieldMetadataType) -> &'static [CompositeProperty] {
    match field_type {
        FieldMetadataType::Links => &[
            CompositeProperty {
                name: "primaryLinkLabel",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "primaryLinkUrl",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: true,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "secondaryLinks",
                field_type: FieldMetadataType::RawJson,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
        ],
        FieldMetadataType::Currency => &[
            CompositeProperty {
                name: "amountMicros",
                field_type: FieldMetadataType::Numeric,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "currencyCode",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
        ],
        FieldMetadataType::FullName => &[
            CompositeProperty {
                name: "firstName",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "lastName",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
        ],
        FieldMetadataType::Address => &[
            CompositeProperty {
                name: "addressStreet1",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "addressStreet2",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "addressCity",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "addressPostcode",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "addressState",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "addressCountry",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "addressLat",
                field_type: FieldMetadataType::Numeric,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "addressLng",
                field_type: FieldMetadataType::Numeric,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
        ],
        FieldMetadataType::Actor => &[
            CompositeProperty {
                name: "source",
                field_type: FieldMetadataType::Select,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &FIELD_ACTOR_SOURCE_OPTIONS,
            },
            CompositeProperty {
                name: "workspaceMemberId",
                field_type: FieldMetadataType::Uuid,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "name",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "context",
                field_type: FieldMetadataType::RawJson,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
        ],
        FieldMetadataType::Emails => &[
            CompositeProperty {
                name: "primaryEmail",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: true,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "additionalEmails",
                field_type: FieldMetadataType::RawJson,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
        ],
        FieldMetadataType::Phones => &[
            CompositeProperty {
                name: "primaryPhoneNumber",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: true,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "primaryPhoneCountryCode",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: true,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "primaryPhoneCallingCode",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: true,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "additionalPhones",
                field_type: FieldMetadataType::RawJson,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
        ],
        FieldMetadataType::RichText => &[
            CompositeProperty {
                name: "blocknote",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "markdown",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
            CompositeProperty {
                name: "tiptap",
                field_type: FieldMetadataType::Text,
                is_included_in_unique_constraint: false,
                is_array: false,
                options: &[],
            },
        ],
        _ => &[],
    }
}

fn is_null_equivalent_text_value(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::String(text)) if text == "''" || text.is_empty())
}

fn is_null_equivalent_array_value(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Array(items)) => items.is_empty(),
        Some(Value::Null) => true,
        _ => false,
    }
}

fn text_property(source: &Map<String, Value>, key: &str) -> Value {
    let value = source.get(key);

    if is_null_equivalent_text_value(value) {
        Value::Null
    } else {
        value.cloned().unwrap_or(Value::Null)
    }
}

fn passthrough_property(source: &Map<String, Value>, key: &str) -> Value {
    source.get(key).cloned().unwrap_or(Value::Null)
}

fn array_property(source: &Map<String, Value>, key: &str) -> Value {
    let value = source.get(key);

    if is_null_equivalent_array_value(value) {
        Value::Null
    } else {
        value.cloned().unwrap_or(Value::Null)
    }
}

fn normalize(
    build: impl Fn(&Map<String, Value>) -> Vec<(&'static str, Value)>,
    default_value: Option<&Value>,
) -> Option<Map<String, Value>> {
    let source = match default_value {
        Some(Value::Object(map)) => map,
        _ => return None,
    };

    let normalized = build(source);

    if normalized.iter().all(|(_, value)| value.is_null()) {
        return None;
    }

    let mut map = Map::new();
    for (key, value) in normalized {
        map.insert(key.to_string(), value);
    }

    Some(map)
}

pub fn nullify_empty_composite_default_value(
    default_value: Option<&Value>,
    field_type: FieldMetadataType,
) -> Option<Map<String, Value>> {
    match field_type {
        FieldMetadataType::Phones => normalize(
            |source| {
                vec![
                    (
                        "primaryPhoneNumber",
                        text_property(source, "primaryPhoneNumber"),
                    ),
                    (
                        "primaryPhoneCountryCode",
                        text_property(source, "primaryPhoneCountryCode"),
                    ),
                    (
                        "primaryPhoneCallingCode",
                        text_property(source, "primaryPhoneCallingCode"),
                    ),
                    (
                        "additionalPhones",
                        array_property(source, "additionalPhones"),
                    ),
                ]
            },
            default_value,
        ),
        FieldMetadataType::Emails => normalize(
            |source| {
                vec![
                    ("primaryEmail", text_property(source, "primaryEmail")),
                    (
                        "additionalEmails",
                        array_property(source, "additionalEmails"),
                    ),
                ]
            },
            default_value,
        ),
        FieldMetadataType::Links => normalize(
            |source| {
                vec![
                    (
                        "primaryLinkLabel",
                        text_property(source, "primaryLinkLabel"),
                    ),
                    ("primaryLinkUrl", text_property(source, "primaryLinkUrl")),
                    ("secondaryLinks", array_property(source, "secondaryLinks")),
                ]
            },
            default_value,
        ),
        FieldMetadataType::Address => normalize(
            |source| {
                vec![
                    ("addressStreet1", text_property(source, "addressStreet1")),
                    ("addressStreet2", text_property(source, "addressStreet2")),
                    ("addressCity", text_property(source, "addressCity")),
                    ("addressState", text_property(source, "addressState")),
                    ("addressCountry", text_property(source, "addressCountry")),
                    ("addressPostcode", text_property(source, "addressPostcode")),
                    ("addressLat", passthrough_property(source, "addressLat")),
                    ("addressLng", passthrough_property(source, "addressLng")),
                ]
            },
            default_value,
        ),
        FieldMetadataType::FullName => normalize(
            |source| {
                vec![
                    ("firstName", text_property(source, "firstName")),
                    ("lastName", text_property(source, "lastName")),
                ]
            },
            default_value,
        ),
        FieldMetadataType::Actor => normalize(
            |source| {
                vec![
                    ("source", passthrough_property(source, "source")),
                    (
                        "workspaceMemberId",
                        passthrough_property(source, "workspaceMemberId"),
                    ),
                    ("name", text_property(source, "name")),
                    ("context", passthrough_property(source, "context")),
                ]
            },
            default_value,
        ),
        FieldMetadataType::Currency => normalize(
            |source| {
                vec![
                    ("amountMicros", passthrough_property(source, "amountMicros")),
                    ("currencyCode", text_property(source, "currencyCode")),
                ]
            },
            default_value,
        ),
        FieldMetadataType::RichText => normalize(
            |source| {
                vec![
                    ("blocknote", text_property(source, "blocknote")),
                    ("markdown", text_property(source, "markdown")),
                    ("tiptap", text_property(source, "tiptap")),
                ]
            },
            default_value,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nullify_empty_phones_defaults_to_none_when_everything_is_empty() {
        let default_value = json!({
            "primaryPhoneNumber": "''",
            "primaryPhoneCountryCode": "'US'",
            "primaryPhoneCallingCode": "'+1'",
            "additionalPhones": Value::Null
        });

        let normalized =
            nullify_empty_composite_default_value(Some(&default_value), FieldMetadataType::Phones)
                .unwrap();

        assert_eq!(normalized.get("primaryPhoneNumber"), Some(&Value::Null));
        assert_eq!(
            normalized.get("primaryPhoneCountryCode"),
            Some(&Value::String("'US'".to_string()))
        );
        assert_eq!(
            normalized.get("primaryPhoneCallingCode"),
            Some(&Value::String("'+1'".to_string()))
        );
        assert_eq!(normalized.get("additionalPhones"), Some(&Value::Null));

        let all_empty = json!({
            "primaryPhoneNumber": "",
            "primaryPhoneCountryCode": "",
            "primaryPhoneCallingCode": "",
            "additionalPhones": Value::Null
        });

        assert_eq!(
            nullify_empty_composite_default_value(Some(&all_empty), FieldMetadataType::Phones),
            None
        );
    }

    #[test]
    fn nullify_keeps_actor_source_even_when_the_rest_is_empty() {
        let default_value = json!({
            "source": "'MANUAL'",
            "workspaceMemberId": Value::Null,
            "name": "''",
            "context": Value::Null
        });

        let normalized =
            nullify_empty_composite_default_value(Some(&default_value), FieldMetadataType::Actor)
                .unwrap();

        assert_eq!(
            normalized.get("source"),
            Some(&Value::String("'MANUAL'".to_string()))
        );
        assert_eq!(normalized.get("name"), Some(&Value::Null));
    }

    #[test]
    fn nullify_normalizes_currency_and_full_name() {
        let currency = json!({ "amountMicros": 100000000, "currencyCode": "''" });
        let normalized =
            nullify_empty_composite_default_value(Some(&currency), FieldMetadataType::Currency)
                .unwrap();
        assert_eq!(normalized.get("amountMicros"), Some(&json!(100000000)));
        assert_eq!(normalized.get("currencyCode"), Some(&Value::Null));

        assert_eq!(
            nullify_empty_composite_default_value(
                Some(&json!({ "firstName": "''", "lastName": Value::Null })),
                FieldMetadataType::FullName
            ),
            None
        );
    }

    #[test]
    fn nullify_returns_none_for_null_and_missing_defaults() {
        assert_eq!(
            nullify_empty_composite_default_value(None, FieldMetadataType::Phones),
            None
        );
        assert_eq!(
            nullify_empty_composite_default_value(Some(&Value::Null), FieldMetadataType::Links),
            None
        );
        assert_eq!(
            nullify_empty_composite_default_value(
                Some(&json!("not an object")),
                FieldMetadataType::Links
            ),
            None
        );
    }

    #[test]
    fn composite_type_membership_matches_upstream() {
        for field_type in COMPOSITE_TYPES {
            assert!(is_composite_field_metadata_type(field_type));
        }
        assert!(!is_composite_field_metadata_type(FieldMetadataType::Text));
        assert!(!is_composite_field_metadata_type(FieldMetadataType::Uuid));
        assert!(!is_composite_field_metadata_type(
            FieldMetadataType::Relation
        ));
    }

    #[test]
    fn actor_source_options_are_the_field_actor_source_values_in_order() {
        assert_eq!(FIELD_ACTOR_SOURCE_OPTIONS.len(), 10);
        assert_eq!(FIELD_ACTOR_SOURCE_OPTIONS[0], "EMAIL");
        assert_eq!(FIELD_ACTOR_SOURCE_OPTIONS[5], "IMPORT");
        assert_eq!(FIELD_ACTOR_SOURCE_OPTIONS[9], "APPLICATION");
    }
}
