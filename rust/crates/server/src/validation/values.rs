use serde_json::Value;

pub fn is_null(value: &Value) -> bool {
    value.is_null()
}

pub fn utf16_length(value: &str) -> usize {
    value.chars().map(|ch| ch.len_utf16()).sum()
}

pub fn is_js_whitespace(value: char) -> bool {
    matches!(
        value,
        '\t' | '\n'
            | '\u{0b}'
            | '\u{0c}'
            | '\r'
            | ' '
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

pub fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

pub fn js_typeof(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) | Value::Object(_) => "object",
    }
}

pub fn object_entries(value: &Value) -> Vec<(String, Value)> {
    match value {
        Value::Object(object) => object
            .iter()
            .map(|(key, item)| (key.clone(), item.clone()))
            .collect(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item.clone()))
            .collect(),
        Value::String(text) => text
            .encode_utf16()
            .enumerate()
            .map(|(index, unit)| (index.to_string(), Value::String(char::from_u32(unit as u32).unwrap_or('\u{fffd}').to_string())))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn is_non_empty_string(value: &Value) -> bool {
    value.as_str().is_some_and(|text| !text.is_empty())
}

pub fn is_string(value: &Value) -> bool {
    value.is_string()
}

pub fn is_boolean(value: &Value) -> bool {
    value.is_boolean()
}

pub fn is_number(value: &Value) -> bool {
    value.is_number()
}

pub fn is_object(value: &Value) -> bool {
    js_typeof(value) == "object" && !value.is_null()
}

pub fn is_plain_object(value: &Value) -> bool {
    matches!(value, Value::Object(_))
}

pub fn js_to_number(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(true) => 1.0,
        Value::Bool(false) => 0.0,
        Value::Number(number) => number.as_f64().unwrap_or(f64::NAN),
        Value::String(text) => js_string_to_number(text),
        Value::Array(items) => match items.len() {
            0 => 0.0,
            1 => js_to_number(&items[0]),
            _ => f64::NAN,
        },
        Value::Object(_) => f64::NAN,
    }
}

pub fn js_string_to_number(text: &str) -> f64 {
    let trimmed = js_trim(text);

    if trimmed.is_empty() {
        return 0.0;
    }

    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(|value| value as f64)
            .unwrap_or(f64::NAN);
    }

    if let Some(octal) = trimmed.strip_prefix("0o").or_else(|| trimmed.strip_prefix("0O")) {
        return u64::from_str_radix(octal, 8)
            .map(|value| value as f64)
            .unwrap_or(f64::NAN);
    }

    if let Some(binary) = trimmed.strip_prefix("0b").or_else(|| trimmed.strip_prefix("0B")) {
        return u64::from_str_radix(binary, 2)
            .map(|value| value as f64)
            .unwrap_or(f64::NAN);
    }

    let lowered = trimmed.to_ascii_lowercase();

    match lowered.as_str() {
        "infinity" | "+infinity" => f64::INFINITY,
        "-infinity" => f64::NEG_INFINITY,
        "nan" => f64::NAN,
        _ => trimmed
            .parse::<f64>()
            .ok()
            .or_else(|| lowered.trim_start_matches('+').parse::<f64>().ok())
            .unwrap_or(f64::NAN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_guard_follows_the_typeof_object_check() {
        assert!(is_object(&json!({ "a": 1 })));
        assert!(is_object(&json!([1, 2])));
        assert!(!is_object(&json!("a")));
        assert!(!is_object(&Value::Null));
        assert!(!is_plain_object(&json!([1])));
    }

    #[test]
    fn numeric_coercion_follows_the_javascript_number_function() {
        assert_eq!(js_to_number(&json!("123")), 123.0);
        assert_eq!(js_to_number(&json!("")), 0.0);
        assert!(js_to_number(&json!("abc")).is_nan());
        assert_eq!(js_to_number(&json!(true)), 1.0);
        assert_eq!(js_to_number(&json!(false)), 0.0);
        assert_eq!(js_to_number(&json!([])), 0.0);
        assert_eq!(js_to_number(&json!([5])), 5.0);
        assert!(js_to_number(&json!([5, 6])).is_nan());
        assert!(js_to_number(&json!({})).is_nan());
        assert_eq!(js_to_number(&json!("1e3")), 1000.0);
        assert_eq!(js_to_number(&json!("0x10")), 16.0);
        assert!(js_to_number(&json!("Infinity")).is_infinite());
    }

    #[test]
    fn entries_follow_object_entries() {
        assert_eq!(
            object_entries(&json!({ "b": 1, "a": 2 })),
            vec![
                (String::from("b"), json!(1)),
                (String::from("a"), json!(2)),
            ]
        );
        assert_eq!(
            object_entries(&json!([10, 20])),
            vec![
                (String::from("0"), json!(10)),
                (String::from("1"), json!(20)),
            ]
        );
        assert_eq!(object_entries(&json!("")).len(), 0);
        assert_eq!(object_entries(&json!("{}"))[0].0, "0");
        assert_eq!(object_entries(&json!("{}")).len(), 2);
        assert_eq!(object_entries(&json!(5)).len(), 0);
        assert_eq!(object_entries(&Value::Null).len(), 0);
    }

    #[test]
    fn javascript_whitespace_set_excludes_the_next_line_character() {
        assert!(is_js_whitespace('\u{a0}'));
        assert!(is_js_whitespace('\u{feff}'));
        assert!(is_js_whitespace('\u{3000}'));
        assert!(!is_js_whitespace('\u{85}'));
        assert_eq!(utf16_length("a😀b"), 4);
        assert_eq!(utf16_length("abc"), 3);
        assert_eq!(js_trim("\u{feff} x \u{85}"), "x \u{85}");
        assert_eq!(js_typeof(&json!([])), "object");
        assert_eq!(js_typeof(&Value::Null), "null");
    }
}
