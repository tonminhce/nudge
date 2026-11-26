use serde_json::json;
use serde_json::Value;

use super::errors::GraphQlError;

pub fn compute_deterministic_uuid(namespace: &str, value: &str, name: &str) -> String {
    let input = format!("{namespace}:{value}");

    let namespace_bytes = uuid::Uuid::parse_str(name)
        .unwrap_or_else(|_| uuid::Uuid::nil())
        .into_bytes();

    let mut data = namespace_bytes.to_vec();

    data.extend_from_slice(input.as_bytes());

    let sha1_output = sha1_bytes(&data);

    let mut uuid_bytes = [0u8; 16];

    uuid_bytes[..16].copy_from_slice(&sha1_output[..16]);

    uuid_bytes[6] = (uuid_bytes[6] & 0x0f) | 0x50;
    uuid_bytes[8] = (uuid_bytes[8] & 0x3f) | 0x80;

    uuid::Uuid::from_bytes(uuid_bytes).to_string()
}

fn sha1_bytes(input: &[u8]) -> [u8; 20] {
    use sha1::Digest;

    let mut hasher = sha1::Sha1::new();

    hasher.update(input);

    let digest = hasher.finalize();
    let mut output = [0u8; 20];

    output.copy_from_slice(&digest);

    output
}

fn trim_and_collapse_whitespaces(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut pending_space = false;

    for character in value.trim().chars() {
        if character.is_whitespace() {
            pending_space = true;
            continue;
        }

        if pending_space && !result.is_empty() {
            result.push(' ');
        }

        pending_space = false;
        result.push(character);
    }

    result
}

pub fn sanitize_string_fields(object: &mut serde_json::Map<String, Value>, fields: &[&str]) {
    for field in fields {
        let Some(trimmed) = object
            .get(*field)
            .and_then(Value::as_str)
            .map(trim_and_collapse_whitespaces)
        else {
            continue;
        };

        object.insert((*field).to_string(), Value::String(trimmed));
    }
}

pub fn require_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, GraphQlError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| GraphQlError::validation(format!("Field \"{field}\" must be a string")))
}

pub fn optional_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<String>, GraphQlError> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => Ok(Some(raw.clone())),
        Some(_) => Err(GraphQlError::validation(format!(
            "Field \"{field}\" must be a string"
        ))),
    }
}

pub fn optional_bool(object: &serde_json::Map<String, Value>, field: &str) -> Option<bool> {
    object.get(field).and_then(Value::as_bool)
}

pub fn optional_value(object: &serde_json::Map<String, Value>, field: &str) -> Option<Value> {
    match object.get(field) {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.clone()),
    }
}

pub fn require_uuid(value: &str, field: &str) -> Result<String, GraphQlError> {
    uuid::Uuid::parse_str(value)
        .map(|parsed| parsed.to_string())
        .map_err(|_| GraphQlError::validation(format!("{field} must be a valid UUID")))
}

pub fn require_object_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    field: &str,
) -> Result<&'a Value, GraphQlError> {
    object
        .get(field)
        .filter(|value| !value.is_null())
        .ok_or_else(|| GraphQlError::validation(format!("Field \"{field}\" is required")))
}

pub fn is_composite_field_type(field_type: &str) -> bool {
    matches!(
        field_type,
        "ADDRESS" | "CURRENCY" | "EMAILS" | "FULL_NAME" | "LINKS" | "PHONES"
    )
}

pub fn is_rating_option_value(value: &str) -> bool {
    let Some(digit) = value.strip_prefix("RATING_") else {
        return false;
    };

    matches!(digit, "1" | "2" | "3" | "4" | "5")
}

pub fn sanitize_select_options(options: &Value) -> Value {
    let Some(entries) = options.as_array() else {
        return Value::Array(Vec::new());
    };

    let sanitized = entries
        .iter()
        .filter_map(|entry| {
            let object = entry.as_object()?;
            let mut copy = object.clone();

            sanitize_string_fields(&mut copy, &["label", "value", "id", "color", "description"]);

            let color = copy
                .get("color")
                .cloned()
                .filter(|value| !value.is_null())
                .unwrap_or_else(|| json!("gray"));

            copy.insert("color".to_string(), color);

            if let Some(description) = copy.get("description").cloned() {
                if description.as_str().is_none_or(str::is_empty) {
                    copy.insert("description".to_string(), Value::Null);
                }
            }

            Some(Value::Object(copy))
        })
        .collect();

    Value::Array(sanitized)
}

pub fn rating_options() -> Value {
    Value::Array(
        (1..=5)
            .map(|value| {
                json!({
                    "id": uuid::Uuid::new_v4().to_string(),
                    "label": value.to_string(),
                    "value": format!("RATING_{value}"),
                    "position": (value - 1) as i64,
                })
            })
            .collect(),
    )
}

pub fn nullify_empty_composite_default_value(field_type: &str, value: &Value) -> Value {
    if !is_composite_field_type(field_type) {
        return value.clone();
    }

    let Some(object) = value.as_object() else {
        return value.clone();
    };

    let all_properties_empty = !object.is_empty()
        && object.values().all(|entry| match entry {
            Value::Null => true,
            Value::String(text) => text.is_empty(),
            _ => false,
        });

    if all_properties_empty {
        return Value::Null;
    }

    value.clone()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn deterministic_uuids_match_upstream_vectors() {
        assert_eq!(
            compute_deterministic_uuid(
                "fieldMetadata",
                "22222222-2222-4222-8222-222222222222:createdAt",
                "11111111-1111-4111-8111-111111111111"
            ),
            "b5cc95b6-e78f-531f-b74e-25bf891e8ec7"
        );

        assert_eq!(
            compute_deterministic_uuid("nope", "nope", "00000000-0000-0000-0000-000000000000"),
            "b2b4c56b-6e84-5575-9249-36effdd3827b"
        );
    }

    #[test]
    fn string_sanitization_collapses_inner_whitespace() {
        let mut object = json!({ "name": "  hello   world  " })
            .as_object()
            .cloned()
            .unwrap();

        sanitize_string_fields(&mut object, &["name"]);

        assert_eq!(object.get("name"), Some(&json!("hello world")));
    }

    #[test]
    fn whitespace_collapse_keeps_single_spacing() {
        assert_eq!(trim_and_collapse_whitespaces("  a   b  c  "), "a b c");
        assert_eq!(trim_and_collapse_whitespaces("plain"), "plain");
        assert_eq!(trim_and_collapse_whitespaces("   "), "");
    }

    #[test]
    fn select_options_get_default_colors_and_null_descriptions() {
        let options = json!([
            {"label": " First ", "value": "FIRST", "description": ""},
            {"label": "Second", "value": "SECOND", "color": "blue", "description": " kept "}
        ]);

        let sanitized = sanitize_select_options(&options);
        let entries = sanitized.as_array().unwrap();

        assert_eq!(entries[0]["color"], json!("gray"));
        assert_eq!(entries[0]["description"], Value::Null);
        assert_eq!(entries[0]["label"], json!("First"));
        assert_eq!(entries[1]["color"], json!("blue"));
        assert_eq!(entries[1]["description"], json!("kept"));
    }

    #[test]
    fn composite_default_value_nullification_follows_the_upstream_rule() {
        assert_eq!(
            nullify_empty_composite_default_value(
                "FULL_NAME",
                &json!({"firstName": "", "lastName": ""})
            ),
            Value::Null
        );
        assert_eq!(
            nullify_empty_composite_default_value(
                "FULL_NAME",
                &json!({"firstName": "Ada", "lastName": "Lovelace"})
            ),
            json!({"firstName": "Ada", "lastName": "Lovelace"})
        );
        assert_eq!(
            nullify_empty_composite_default_value("TEXT", &json!("")),
            json!("")
        );
    }

    #[test]
    fn rating_options_span_one_to_five() {
        let options = rating_options();
        let entries = options.as_array().unwrap();

        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0]["value"], json!("RATING_1"));
        assert_eq!(entries[4]["position"], json!(4));
        assert!(is_rating_option_value("RATING_3"));
        assert!(!is_rating_option_value("RATING_9"));
    }

    #[test]
    fn uuid_validation_rejects_non_uuids() {
        assert!(require_uuid("not-a-uuid", "id").is_err());
        let parsed = require_uuid("22222222-2222-4222-8222-222222222222", "id").unwrap();

        assert_eq!(parsed, "22222222-2222-4222-8222-222222222222");
    }
}
