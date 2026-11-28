use serde_json::Value;

use crate::validation::date::is_valid_date_format;
use crate::validation::date::parse_to_instant_or_throw;
use crate::validation::error::ValidationError;
use crate::validation::error::ValidationResult;
use crate::validation::inspect::inspect;
use crate::validation::inspect::is_js_finite;
use crate::validation::inspect::render_f64;
use crate::validation::net::is_valid_uuid;
use crate::validation::values::is_boolean;
use crate::validation::values::is_non_empty_string;
use crate::validation::values::is_null;
use crate::validation::values::is_number;
use crate::validation::values::is_object;
use crate::validation::values::is_string;
use crate::validation::values::js_to_number;

pub fn validate_text_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    if !is_string(value) && !is_null(value) {
        let inspected = inspect(value);

        return Err(ValidationError::new(
            format!("Invalid string value {inspected} for text field \"{field_name}\""),
            format!("Invalid value: \"{inspected}\""),
        ));
    }

    Ok(())
}

fn number_error(rendered: &str, field_name: &str) -> ValidationError {
    ValidationError::new(
        format!("Invalid number value {rendered} for field \"{field_name}\""),
        format!("Invalid value for number: \"{rendered}\""),
    )
}

pub fn validate_number_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    if is_null(value) {
        return Ok(());
    }

    if is_number(value) && value.as_number().is_some_and(is_js_finite) {
        return Ok(());
    }

    Err(number_error(&inspect(value), field_name))
}

pub fn validate_numeric_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    if value.as_str().is_some_and(|text| text.is_empty()) || is_null(value) {
        return Ok(());
    }

    let coerced = js_to_number(value);

    if coerced.is_finite() {
        return Ok(());
    }

    Err(number_error(&render_f64(coerced), field_name))
}

pub fn validate_boolean_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    if !is_boolean(value) && !is_null(value) {
        let inspected = inspect(value);

        return Err(ValidationError::new(
            format!("Invalid boolean value {inspected} for field \"{field_name}\""),
            format!("Invalid value: \"{inspected}\""),
        ));
    }

    Ok(())
}

pub fn validate_uuid_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    let invalid = (!is_non_empty_string(value) && !is_null(value))
        || (is_non_empty_string(value) && !is_valid_uuid(value.as_str().unwrap_or_default()));

    if invalid {
        let inspected = inspect(value);

        return Err(ValidationError::new(
            format!("Invalid UUID value {inspected} for field \"{field_name}\""),
            format!("Invalid value for UUID: \"{inspected}\""),
        ));
    }

    Ok(())
}

pub fn validate_array_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    if is_null(value) || value.is_string() {
        return Ok(());
    }

    let invalid = match value {
        Value::Array(items) => items.iter().any(|item| !item.is_string()),
        _ => true,
    };

    if invalid {
        let inspected = inspect(value);

        return Err(ValidationError::new(
            format!(
                "Invalid value {inspected} for field \"{field_name} - Array values need to be string\""
            ),
            format!("Invalid value: \"{inspected}\""),
        ));
    }

    Ok(())
}

pub fn validate_raw_json_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    if is_null(value) {
        return Ok(());
    }

    if let Some(text) = value.as_str() {
        if serde_json::from_str::<Value>(text).is_err() {
            let inspected = inspect(value);

            return Err(ValidationError::new(
                format!("Invalid object value {inspected} for field \"{field_name}\""),
                String::from("Invalid value for JSON."),
            ));
        }

        return Ok(());
    }

    if !is_object(value) {
        let inspected = inspect(value);

        return Err(ValidationError::new(
            format!("Invalid object value {inspected} for field \"{field_name}\""),
            format!("Invalid value for JSON: \"{inspected}\""),
        ));
    }

    Ok(())
}

pub fn validate_date_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    if is_null(value) {
        return Ok(());
    }

    if value
        .as_str()
        .is_some_and(|text| is_valid_date_format(text))
    {
        return Ok(());
    }

    let inspected = inspect(value);

    Err(ValidationError::new(
        format!(
            "Invalid value {inspected} for date field \"{field_name}\". Expected format: 'YYYY-MM-DD'"
        ),
        format!(
            "Invalid value for date: \"{inspected}\". Expected format: 'YYYY-MM-DD'"
        ),
    ))
}

pub fn validate_date_time_field(value: &Value, field_name: &str) -> ValidationResult<()> {
    if is_null(value) {
        return Ok(());
    }

    if value
        .as_str()
        .is_some_and(|text| parse_to_instant_or_throw(text).is_some())
    {
        return Ok(());
    }

    let inspected = inspect(value);

    Err(ValidationError::new(
        format!(
            "Invalid value {inspected} for date-time field \"{field_name}\". Expected format: 'YYYY-MM-DDTHH:mm:ssZ'"
        ),
        format!(
            "Invalid value for date-time: \"{inspected}\". Expected format: 'YYYY-MM-DDTHH:mm:ssZ'"
        ),
    ))
}

pub fn validate_rating_and_select_field(
    value: &Value,
    field_name: &str,
    options: Option<&[String]>,
) -> ValidationResult<()> {
    validate_text_field(value, field_name)?;

    let Some(options) = options else {
        return Err(ValidationError::standard(format!(
            "Invalid options for field \"{field_name}\""
        )));
    };

    if !is_null(value) {
        let text = value.as_str().unwrap_or_default();

        if !options.iter().any(|option| option == text) {
            return Err(ValidationError::new(
                format!(
                    "Invalid value \"{text}\" for field \"{field_name}\". Valid values are: {}",
                    options.join(", ")
                ),
                format!("Invalid value for field \"{field_name}\""),
            ));
        }
    }

    Ok(())
}

pub fn validate_multi_select_field(
    value: &Value,
    field_name: &str,
    options: Option<&[String]>,
) -> ValidationResult<()> {
    validate_array_field(value, field_name)?;

    if is_null(value) {
        return Ok(());
    }

    let Some(options) = options else {
        return Err(ValidationError::standard(format!(
            "Invalid options for field \"{field_name}\""
        )));
    };

    let items: Vec<&str> = match value {
        Value::Array(entries) => entries.iter().filter_map(Value::as_str).collect(),
        Value::String(text) => vec![text.as_str()],
        _ => Vec::new(),
    };

    if items
        .iter()
        .any(|item| !options.iter().any(|option| option == item))
    {
        let inspected = inspect(value);

        return Err(ValidationError::new(
            format!("Invalid value {inspected} for multi select field \"{field_name}\""),
            format!("Invalid value for multi-select: \"{inspected}\""),
        ));
    }

    Ok(())
}

pub fn validate_overridden_position_field(
    value: &Value,
    field_name: &str,
) -> ValidationResult<()> {
    let invalid = !value.is_number()
        || value
            .as_f64()
            .map(|inner| !inner.is_finite())
            .unwrap_or(false);

    if invalid {
        let inspected = inspect(value);

        return Err(ValidationError::new(
            format!("Invalid position value {inspected} for field \"{field_name}\""),
            format!("Invalid value for position: \"{inspected}\""),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::error::STANDARD_ERROR_MESSAGE;
    use serde_json::json;

    fn message(result: ValidationResult<()>) -> String {
        result.expect_err("validation error").message().to_string()
    }

    fn friendly(result: ValidationResult<()>) -> String {
        result.expect_err("validation error").user_friendly_message().to_string()
    }

    #[test]
    fn text_rejects_non_strings_with_the_inspected_value() {
        assert!(validate_text_field(&json!("hello"), "name").is_ok());
        assert!(validate_text_field(&Value::Null, "name").is_ok());
        assert_eq!(
            message(validate_text_field(&json!(42), "name")),
            "Invalid string value 42 for text field \"name\""
        );
        assert_eq!(
            friendly(validate_text_field(&json!([1]), "name")),
            "Invalid value: \"[ 1 ]\""
        );
    }

    #[test]
    fn number_rejects_strings_even_when_they_parse() {
        assert!(validate_number_field(&json!(12), "salary").is_ok());
        assert!(validate_number_field(&Value::Null, "salary").is_ok());
        assert_eq!(
            message(validate_number_field(&json!("12"), "salary")),
            "Invalid number value '12' for field \"salary\""
        );
        assert_eq!(
            friendly(validate_number_field(&json!(true), "salary")),
            "Invalid value for number: \"true\""
        );
    }

    #[test]
    fn numeric_coerces_before_reporting_and_accepts_the_empty_string() {
        assert!(validate_numeric_field(&json!(""), "size").is_ok());
        assert!(validate_numeric_field(&json!("3.5"), "size").is_ok());
        assert!(validate_numeric_field(&Value::Null, "size").is_ok());
        assert_eq!(
            message(validate_numeric_field(&json!("abc"), "size")),
            "Invalid number value NaN for field \"size\""
        );
        assert_eq!(
            message(validate_numeric_field(&json!([1, 2]), "size")),
            "Invalid number value NaN for field \"size\""
        );
    }

    #[test]
    fn boolean_and_uuid_follow_the_reference_messages() {
        assert_eq!(
            message(validate_boolean_field(&json!("yes"), "active")),
            "Invalid boolean value 'yes' for field \"active\""
        );
        assert!(validate_boolean_field(&json!(false), "active").is_ok());
        assert!(validate_uuid_field(&json!("20202020-0000-4000-8000-000000000001"), "id").is_ok());
        assert!(validate_uuid_field(&Value::Null, "id").is_ok());
        assert!(validate_uuid_field(&json!(""), "id").is_err());
        assert_eq!(
            message(validate_uuid_field(&json!("nope"), "id")),
            "Invalid UUID value 'nope' for field \"id\""
        );
    }

    #[test]
    fn array_keeps_the_joined_field_and_message_suffix() {
        assert!(validate_array_field(&json!(["a", "b"]), "tags").is_ok());
        assert!(validate_array_field(&json!("a"), "tags").is_ok());
        assert_eq!(
            message(validate_array_field(&json!([1]), "tags")),
            "Invalid value [ 1 ] for field \"tags - Array values need to be string\""
        );
    }

    #[test]
    fn raw_json_reports_strings_and_scalars_differently() {
        assert!(validate_raw_json_field(&json!({ "a": 1 }), "meta").is_ok());
        assert!(validate_raw_json_field(&json!("[1]"), "meta").is_ok());
        assert_eq!(
            message(validate_raw_json_field(&json!("nope"), "meta")),
            "Invalid object value 'nope' for field \"meta\""
        );
        assert_eq!(
            friendly(validate_raw_json_field(&json!("nope"), "meta")),
            "Invalid value for JSON."
        );
        assert_eq!(
            friendly(validate_raw_json_field(&json!(7), "meta")),
            "Invalid value for JSON: \"7\""
        );
    }

    #[test]
    fn date_and_date_time_carry_their_expected_formats() {
        assert!(validate_date_field(&json!("2024-01-01"), "due").is_ok());
        assert_eq!(
            message(validate_date_field(&json!("31/12/2024"), "due")),
            "Invalid value '31/12/2024' for date field \"due\". Expected format: 'YYYY-MM-DD'"
        );
        assert!(validate_date_time_field(&json!("2024-01-01T10:00:00Z"), "at").is_ok());
        assert_eq!(
            message(validate_date_time_field(&json!("tomorrow"), "at")),
            "Invalid value 'tomorrow' for date-time field \"at\". Expected format: 'YYYY-MM-DDTHH:mm:ssZ'"
        );
    }

    #[test]
    fn select_lists_the_valid_values_and_missing_options_are_standard() {
        let options = ["NEW".to_string(), "WON".to_string()];

        assert!(validate_rating_and_select_field(&json!("WON"), "status", Some(&options)).is_ok());
        assert!(validate_rating_and_select_field(&Value::Null, "status", Some(&options)).is_ok());
        assert_eq!(
            message(validate_rating_and_select_field(
                &json!("LOST"),
                "status",
                Some(&options)
            )),
            "Invalid value \"LOST\" for field \"status\". Valid values are: NEW, WON"
        );
        assert_eq!(
            friendly(validate_rating_and_select_field(
                &json!("LOST"),
                "status",
                Some(&options)
            )),
            "Invalid value for field \"status\""
        );
        assert_eq!(
            message(validate_rating_and_select_field(&json!("LOST"), "status", None)),
            "Invalid options for field \"status\""
        );
        assert_eq!(
            friendly(validate_rating_and_select_field(&json!("LOST"), "status", None)),
            STANDARD_ERROR_MESSAGE.to_string()
        );
    }

    #[test]
    fn multi_select_inspects_the_original_value() {
        let options = ["A".to_string(), "B".to_string()];

        assert!(validate_multi_select_field(&json!(["A"]), "tags", Some(&options)).is_ok());
        assert!(validate_multi_select_field(&json!("A"), "tags", Some(&options)).is_ok());
        assert!(validate_multi_select_field(&Value::Null, "tags", None).is_ok());
        assert_eq!(
            message(validate_multi_select_field(
                &json!(["A", "Z"]),
                "tags",
                Some(&options)
            )),
            "Invalid value [ 'A', 'Z' ] for multi select field \"tags\""
        );
        assert_eq!(
            friendly(validate_multi_select_field(
                &json!(["A", "Z"]),
                "tags",
                Some(&options)
            )),
            "Invalid value for multi-select: \"[ 'A', 'Z' ]\""
        );
    }

    #[test]
    fn position_requires_a_finite_number() {
        assert!(validate_overridden_position_field(&json!(1), "position").is_ok());
        assert_eq!(
            message(validate_overridden_position_field(&json!("1"), "position")),
            "Invalid position value '1' for field \"position\""
        );
    }
}
