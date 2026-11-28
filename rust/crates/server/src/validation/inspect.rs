use serde_json::Number;
use serde_json::Value;

const COLLAPSE_DEPTH: usize = 3;

pub fn inspect(value: &Value) -> String {
    render(value, 0)
}

fn render(value: &Value, depth: usize) -> String {
    match value {
        Value::Null => String::from("null"),
        Value::Bool(flag) => {
            if *flag {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        Value::Number(number) => render_number(number),
        Value::String(text) => render_string(text),
        Value::Array(items) => {
            if depth >= COLLAPSE_DEPTH {
                return String::from("[Array]");
            }

            if items.is_empty() {
                return String::from("[]");
            }

            let parts = items
                .iter()
                .map(|item| render(item, depth + 1))
                .collect::<Vec<String>>()
                .join(", ");

            format!("[ {parts} ]")
        }
        Value::Object(object) => {
            if depth >= COLLAPSE_DEPTH {
                return String::from("[Object]");
            }

            if object.is_empty() {
                return String::from("{}");
            }

            let parts = object
                .iter()
                .map(|(key, item)| format!("{}: {}", render_key(key), render(item, depth + 1)))
                .collect::<Vec<String>>()
                .join(", ");

            format!("{{ {parts} }}")
        }
    }
}

fn render_key(key: &str) -> String {
    let identifier = key
        .chars()
        .next()
        .is_some_and(|first| first == '_' || first == '$' || first.is_ascii_alphabetic())
        && key
            .chars()
            .all(|ch| ch == '_' || ch == '$' || ch.is_ascii_alphanumeric());

    if identifier {
        key.to_string()
    } else {
        render_string(key)
    }
}

fn render_string(text: &str) -> String {
    let mut body = String::new();

    for ch in text.chars() {
        match ch {
            '\\' => body.push_str("\\\\"),
            '\n' => body.push_str("\\n"),
            '\r' => body.push_str("\\r"),
            '\t' => body.push_str("\\t"),
            '\u{08}' => body.push_str("\\b"),
            '\u{0c}' => body.push_str("\\f"),
            control if (control as u32) < 0x20 || control == '\u{7f}' => {
                body.push_str(&format!("\\x{:02X}", control as u32));
            }
            other => body.push(other),
        }
    }

    let quote = if text.contains('\'') && text.contains('"') {
        '`'
    } else if text.contains('\'') {
        '"'
    } else {
        '\''
    };

    format!("{quote}{body}{quote}")
}

fn render_number(number: &Number) -> String {
    if let Some(integer) = number.as_i64() {
        return render_f64(integer as f64);
    }

    if let Some(unsigned) = number.as_u64() {
        return render_f64(unsigned as f64);
    }

    render_f64(number.as_f64().unwrap_or_default())
}

pub fn render_f64(value: f64) -> String {
    if value.is_nan() {
        return String::from("NaN");
    }

    if value.is_infinite() {
        return if value > 0.0 {
            String::from("Infinity")
        } else {
            String::from("-Infinity")
        };
    }

    if value == 0.0 {
        return if value.is_sign_negative() {
            String::from("-0")
        } else {
            String::from("0")
        };
    }

    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .unwrap_or((scientific.as_str(), "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let digits = mantissa.trim_start_matches(['-', '+']).replace('.', "");

    if exponent >= 21 || exponent < -6 {
        let normalized = if digits.len() == 1 {
            digits.clone()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        let sign = if exponent < 0 { '-' } else { '+' };

        return format!(
            "{mantissa_head}e{sign}{exp}",
            mantissa_head = if negative {
                format!("-{normalized}")
            } else {
                normalized
            },
            exp = exponent.abs()
        );
    }

    if exponent >= 0 {
        let width = exponent as usize + 1;

        return if digits.len() <= width {
            let padded = format!("{}{}", digits, "0".repeat(width - digits.len()));

            if negative {
                format!("-{padded}")
            } else {
                padded
            }
        } else {
            let (head, tail) = digits.split_at(width);
            let rendered = format!("{head}.{tail}");

            if negative {
                format!("-{rendered}")
            } else {
                rendered
            }
        };
    }

    let zeros = (-exponent - 1) as usize;
    let rendered = format!("0.{}{digits}", "0".repeat(zeros));

    if negative {
        format!("-{rendered}")
    } else {
        rendered
    }
}

pub fn is_js_finite(number: &Number) -> bool {
    number
        .as_f64()
        .map(|value| value.is_finite())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rendered(value: Value) -> String {
        inspect(&value)
    }

    #[test]
    fn scalars_match_node_inspect() {
        assert_eq!(rendered(json!("abc")), "'abc'");
        assert_eq!(rendered(json!("")), "''");
        assert_eq!(rendered(json!("it's")), "\"it's\"");
        assert_eq!(rendered(json!("say \"hi\"")), "'say \"hi\"'");
        assert_eq!(rendered(json!("a'b\"c")), "`a'b\"c`");
        assert_eq!(rendered(json!("multi\nline")), "'multi\\nline'");
        assert_eq!(rendered(json!("tab\there")), "'tab\\there'");
        assert_eq!(rendered(json!("ctrl\u{1}a")), "'ctrl\\x01a'");
        assert_eq!(rendered(json!("del\u{7f}b")), "'del\\x7Fb'");
        assert_eq!(rendered(json!("a\\b")), "'a\\\\b'");
        assert_eq!(rendered(json!(123)), "123");
        assert_eq!(rendered(json!(123.45)), "123.45");
        assert_eq!(rendered(json!(true)), "true");
        assert_eq!(rendered(json!(false)), "false");
        assert_eq!(rendered(Value::Null), "null");
    }

    #[test]
    fn containers_match_node_inspect() {
        assert_eq!(rendered(json!([1, 2, 3])), "[ 1, 2, 3 ]");
        assert_eq!(rendered(json!([])), "[]");
        assert_eq!(rendered(json!({})), "{}");
        assert_eq!(rendered(json!({ "key": "value" })), "{ key: 'value' }");
        assert_eq!(rendered(json!({ "a": 1, "b": "x" })), "{ a: 1, b: 'x' }");
        assert_eq!(rendered(json!([{"a": 1}])), "[ { a: 1 } ]");
        assert_eq!(
            rendered(json!({ "a": { "b": { "c": { "d": 1 } } } })),
            "{ a: { b: { c: [Object] } } }"
        );
        assert_eq!(
            rendered(json!([1, [2, [3, [4]]]])),
            "[ 1, [ 2, [ 3, [Array] ] ] ]"
        );
        assert_eq!(
            rendered(json!({ "1a": 1, "a-b": 2, "_x": 3, "$y": 4 })),
            "{ '1a': 1, 'a-b': 2, _x: 3, $y: 4 }"
        );
        assert_eq!(rendered(json!({ "": 1 })), "{ '': 1 }");
    }

    #[test]
    fn numbers_use_the_javascript_representation() {
        assert_eq!(render_f64(1e21), "1e+21");
        assert_eq!(render_f64(1e20), "100000000000000000000");
        assert_eq!(render_f64(1e-7), "1e-7");
        assert_eq!(render_f64(1e-6), "0.000001");
        assert_eq!(render_f64(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(render_f64(-0.0), "-0");
        assert_eq!(render_f64(1.0), "1");
        assert_eq!(render_f64(1.2345678901234568e29), "1.2345678901234568e+29");
    }
}
