#![allow(dead_code)]

use serde_json::Map;
use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;

pub const POSTGRES_DEFAULT_TEXT_FIELD_NULL_EQUIVALENT_VALUE: &str = "";
pub const POSTGRES_DEFAULT_ARRAY_FIELD_NULL_EQUIVALENT_VALUE: &str = "{}";
pub const DEFAULT_TEXT_FIELD_NULL_EQUIVALENT_VALUE: &str = "";
pub const DEFAULT_ARRAY_FIELD_NULL_EQUIVALENT_VALUE: &str = "[]";

pub fn is_null_equivalent_text_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s == DEFAULT_TEXT_FIELD_NULL_EQUIVALENT_VALUE,
        _ => false,
    }
}

pub fn is_null_equivalent_array_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

pub fn find_postgres_default_null_equivalent_value(
    value: &Value,
    field_metadata_type: FieldMetadataType,
    sub_field_key: Option<&str>,
) -> Option<String> {
    match field_metadata_type {
        FieldMetadataType::Text => {
            if is_null_equivalent_text_value(value) || value == &Value::String("NULL".to_string()) {
                Some(POSTGRES_DEFAULT_TEXT_FIELD_NULL_EQUIVALENT_VALUE.to_string())
            } else {
                None
            }
        }
        FieldMetadataType::MultiSelect | FieldMetadataType::Array => {
            if is_null_equivalent_array_value(value) || value == &Value::String("NULL".to_string())
            {
                Some(POSTGRES_DEFAULT_ARRAY_FIELD_NULL_EQUIVALENT_VALUE.to_string())
            } else {
                None
            }
        }
        FieldMetadataType::Actor => match sub_field_key {
            Some("name") => text_equivalent(value),
            _ => None,
        },
        FieldMetadataType::Address => match sub_field_key {
            Some("addressStreet1")
            | Some("addressStreet2")
            | Some("addressCity")
            | Some("addressState")
            | Some("addressPostcode")
            | Some("addressCountry") => text_equivalent(value),
            _ => None,
        },
        FieldMetadataType::Emails => match sub_field_key {
            Some("primaryEmail") => text_equivalent(value),
            Some("additionalEmails") => array_equivalent(value),
            _ => None,
        },
        FieldMetadataType::Links => match sub_field_key {
            Some("primaryLinkUrl") | Some("primaryLinkLabel") => text_equivalent(value),
            Some("secondaryLinks") => array_equivalent(value),
            _ => None,
        },
        FieldMetadataType::Phones => match sub_field_key {
            Some("primaryPhoneNumber")
            | Some("primaryPhoneCountryCode")
            | Some("primaryPhoneCallingCode") => text_equivalent(value),
            Some("additionalPhones") => array_equivalent(value),
            _ => None,
        },
        FieldMetadataType::RichText => match sub_field_key {
            Some("markdown") => text_equivalent(value),
            _ => None,
        },
        FieldMetadataType::FullName => match sub_field_key {
            Some("firstName") | Some("lastName") => text_equivalent(value),
            _ => None,
        },
        _ => None,
    }
}

fn text_equivalent(value: &Value) -> Option<String> {
    if is_null_equivalent_text_value(value) || value == &Value::String("NULL".to_string()) {
        Some(POSTGRES_DEFAULT_TEXT_FIELD_NULL_EQUIVALENT_VALUE.to_string())
    } else {
        None
    }
}

fn array_equivalent(value: &Value) -> Option<String> {
    if is_null_equivalent_array_value(value) || value == &Value::String("NULL".to_string()) {
        Some(POSTGRES_DEFAULT_ARRAY_FIELD_NULL_EQUIVALENT_VALUE.to_string())
    } else {
        None
    }
}

pub fn default_composite_null_equivalents() -> Map<String, Value> {
    let mut map = Map::new();

    for (field_type, properties) in [
        (
            "ADDRESS",
            vec![
                "addressStreet1",
                "addressStreet2",
                "addressCity",
                "addressState",
                "addressPostcode",
                "addressCountry",
            ],
        ),
        ("EMAILS", vec!["primaryEmail", "additionalEmails"]),
        (
            "LINKS",
            vec!["primaryLinkUrl", "primaryLinkLabel", "secondaryLinks"],
        ),
        (
            "PHONES",
            vec![
                "primaryPhoneNumber",
                "primaryPhoneCountryCode",
                "primaryPhoneCallingCode",
                "additionalPhones",
            ],
        ),
        ("FULL_NAME", vec!["firstName", "lastName"]),
        ("ACTOR", vec!["name"]),
        ("RICH_TEXT", vec!["markdown"]),
    ] {
        let mut object = Map::new();

        for property in properties {
            let value = match property {
                "additionalEmails" | "secondaryLinks" | "additionalPhones" => {
                    Value::Array(Vec::new())
                }
                _ => Value::String(POSTGRES_DEFAULT_TEXT_FIELD_NULL_EQUIVALENT_VALUE.to_string()),
            };

            object.insert(property.to_string(), value);
        }

        map.insert(field_type.to_string(), Value::Object(object));
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::flat::FieldMetadataType;
    use serde_json::json;

    #[test]
    fn text_type_widens_null_empty_and_the_null_literal() {
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(null),
                FieldMetadataType::Text,
                None
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(&json!(""), FieldMetadataType::Text, None),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!("NULL"),
                FieldMetadataType::Text,
                None
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!("value"),
                FieldMetadataType::Text,
                None
            ),
            None
        );
    }

    #[test]
    fn array_type_widens_null_empty_array_and_the_null_literal() {
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(null),
                FieldMetadataType::Array,
                None
            ),
            Some("{}".to_string())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(&json!([]), FieldMetadataType::Array, None),
            Some("{}".to_string())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!("NULL"),
                FieldMetadataType::Array,
                None
            ),
            Some("{}".to_string())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(["a"]),
                FieldMetadataType::Array,
                None
            ),
            None
        );
    }

    #[test]
    fn multi_select_shares_the_array_rule() {
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!([]),
                FieldMetadataType::MultiSelect,
                None
            ),
            Some("{}".to_string())
        );
    }

    #[test]
    fn composite_sub_fields_widen_only_text_and_array_sub_fields() {
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Actor,
                Some("name")
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!({}),
                FieldMetadataType::Actor,
                Some("context")
            ),
            None
        );
        for key in [
            "addressStreet1",
            "addressStreet2",
            "addressCity",
            "addressState",
            "addressPostcode",
            "addressCountry",
        ] {
            assert_eq!(
                find_postgres_default_null_equivalent_value(
                    &json!(""),
                    FieldMetadataType::Address,
                    Some(key)
                ),
                Some(String::new()),
                "ADDRESS {key}"
            );
        }
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(1.5),
                FieldMetadataType::Address,
                Some("addressLat")
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Emails,
                Some("primaryEmail")
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!([]),
                FieldMetadataType::Emails,
                Some("additionalEmails")
            ),
            Some("{}".to_string())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Links,
                Some("primaryLinkUrl")
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Links,
                Some("primaryLinkLabel")
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!([]),
                FieldMetadataType::Links,
                Some("secondaryLinks")
            ),
            Some("{}".to_string())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Phones,
                Some("primaryPhoneNumber")
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Phones,
                Some("primaryPhoneCountryCode")
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Phones,
                Some("primaryPhoneCallingCode")
            ),
            Some(String::new())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!([]),
                FieldMetadataType::Phones,
                Some("additionalPhones")
            ),
            Some("{}".to_string())
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!({}),
                FieldMetadataType::RichText,
                Some("blocknote")
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::RichText,
                Some("markdown")
            ),
            Some(String::new())
        );
        for key in ["firstName", "lastName"] {
            assert_eq!(
                find_postgres_default_null_equivalent_value(
                    &json!(""),
                    FieldMetadataType::FullName,
                    Some(key)
                ),
                Some(String::new()),
                "FULL_NAME {key}"
            );
        }
    }

    #[test]
    fn scalar_composite_and_unknown_keys_never_widen() {
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Currency,
                Some("amountMicros")
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Number,
                Some("")
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::DateTime,
                Some("createdAt")
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(""),
                FieldMetadataType::Actor,
                Some("unknown")
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(null),
                FieldMetadataType::Uuid,
                None
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(null),
                FieldMetadataType::Boolean,
                None
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(null),
                FieldMetadataType::Select,
                None
            ),
            None
        );
        assert_eq!(
            find_postgres_default_null_equivalent_value(
                &json!(null),
                FieldMetadataType::RawJson,
                None
            ),
            None
        );
    }

    #[test]
    fn null_equivalent_predicates_follow_the_shared_helpers() {
        assert!(is_null_equivalent_text_value(&json!(null)));
        assert!(is_null_equivalent_text_value(&json!("")));
        assert!(!is_null_equivalent_text_value(&json!("x")));
        assert!(!is_null_equivalent_text_value(&json!(0)));
        assert!(is_null_equivalent_array_value(&json!(null)));
        assert!(is_null_equivalent_array_value(&json!([])));
        assert!(!is_null_equivalent_array_value(&json!(["a"])));
        assert!(!is_null_equivalent_array_value(&json!("")));
    }

    #[test]
    fn composite_default_shapes_are_exposed_for_the_write_side() {
        let defaults = default_composite_null_equivalents();
        assert_eq!(defaults["FULL_NAME"]["firstName"], json!(""));
        assert_eq!(defaults["EMAILS"]["additionalEmails"], json!([]));
        assert_eq!(defaults["ADDRESS"]["addressCity"], json!(""));
        assert_eq!(defaults["ACTOR"]["name"], json!(""));
    }
}
