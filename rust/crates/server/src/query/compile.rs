#![allow(dead_code)]

use std::collections::HashMap;

use serde_json::Map;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    MissingParameter(String),
    InvalidParameter(String),
    MalformedSql(String),
}

impl ExecutorError {
    pub fn code(&self) -> &'static str {
        match self {
            ExecutorError::MissingParameter(_) => "MISSING_PARAMETER",
            ExecutorError::InvalidParameter(_) => "INVALID_PARAMETER",
            ExecutorError::MalformedSql(_) => "MALFORMED_SQL",
        }
    }

    pub fn message(&self) -> String {
        match self {
            ExecutorError::MissingParameter(name) => {
                format!("Parameter \":{name}\" is referenced by the query but was never provided")
            }
            ExecutorError::InvalidParameter(name) => {
                format!("Parameter \":...{name}\" expects an array")
            }
            ExecutorError::MalformedSql(quote) => {
                if quote == "'" {
                    "Unterminated string literal in SQL".to_string()
                } else {
                    "Unterminated quoted identifier in SQL".to_string()
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledStatement {
    pub text: String,
    pub values: Vec<Value>,
}

struct Chars {
    chars: Vec<char>,
}

impl Chars {
    fn new(sql: &str) -> Self {
        Chars {
            chars: sql.chars().collect(),
        }
    }

    fn len(&self) -> usize {
        self.chars.len()
    }

    fn char_at(&self, index: usize) -> Option<char> {
        self.chars.get(index).copied()
    }

    fn slice(&self, start: usize, end: usize) -> String {
        self.chars[start.min(self.len())..end.min(self.len())]
            .iter()
            .collect()
    }

    fn starts_with_at(&self, index: usize, needle: &str) -> bool {
        needle
            .chars()
            .enumerate()
            .all(|(offset, character)| self.char_at(index + offset) == Some(character))
    }
}

fn is_parameter_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

pub fn compile_named_parameters(
    sql: &str,
    parameters: &Map<String, Value>,
) -> Result<CompiledStatement, ExecutorError> {
    let chars = Chars::new(sql);
    let mut values: Vec<Value> = Vec::new();
    let mut position_by_parameter_name: HashMap<String, usize> = HashMap::new();
    let mut text = String::new();
    let mut index = 0usize;

    while index < chars.len() {
        let character = chars.char_at(index).expect("index in range");

        if character == '\'' || character == '"' {
            let closing_index = find_closing_quote_index(&chars, index, character)?;
            text.push_str(&chars.slice(index, closing_index + 1));
            index = closing_index + 1;
            continue;
        }

        if character == ':' && chars.char_at(index + 1) == Some(':') {
            text.push_str("::");
            index += 2;
            continue;
        }

        if character != ':' {
            text.push(character);
            index += 1;
            continue;
        }

        let is_spread = chars.starts_with_at(index, ":...");
        let name_start_index = index + if is_spread { 4 } else { 1 };
        let mut name_end_index = name_start_index;

        while name_end_index < chars.len()
            && is_parameter_name_character(chars.char_at(name_end_index).expect("index in range"))
        {
            name_end_index += 1;
        }

        if name_end_index == name_start_index {
            text.push(character);
            index += 1;
            continue;
        }

        let parameter_name = chars.slice(name_start_index, name_end_index);

        let Some(parameter_value) = parameters.get(&parameter_name) else {
            return Err(ExecutorError::MissingParameter(parameter_name));
        };

        if is_spread {
            let Value::Array(items) = parameter_value else {
                return Err(ExecutorError::InvalidParameter(parameter_name));
            };

            if items.is_empty() {
                text.push_str("NULL");
            } else {
                for (item_index, item) in items.iter().enumerate() {
                    let key = format!(":{parameter_name}__{item_index}");
                    let placeholder = append_value(
                        &mut values,
                        &mut position_by_parameter_name,
                        &key,
                        item.clone(),
                    );

                    if item_index > 0 {
                        text.push_str(", ");
                    }

                    text.push_str(&placeholder);
                }
            }
        } else {
            let placeholder = append_value(
                &mut values,
                &mut position_by_parameter_name,
                &parameter_name,
                parameter_value.clone(),
            );

            text.push_str(&placeholder);
        }

        index = name_end_index;
    }

    Ok(CompiledStatement { text, values })
}

fn append_value(
    values: &mut Vec<Value>,
    position_by_parameter_name: &mut HashMap<String, usize>,
    parameter_name: &str,
    value: Value,
) -> String {
    if let Some(existing_position) = position_by_parameter_name.get(parameter_name) {
        return format!("${existing_position}");
    }

    values.push(value);
    position_by_parameter_name.insert(parameter_name.to_string(), values.len());

    format!("${}", values.len())
}

fn find_closing_quote_index(
    chars: &Chars,
    opening_index: usize,
    quote: char,
) -> Result<usize, ExecutorError> {
    let mut index = opening_index + 1;

    while index < chars.len() {
        if chars.char_at(index) != Some(quote) {
            index += 1;
            continue;
        }

        if chars.char_at(index + 1) == Some(quote) {
            index += 2;
            continue;
        }

        return Ok(index);
    }

    Err(ExecutorError::MalformedSql(quote.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serde_json::Value;

    #[test]
    fn named_parameters_compile_to_the_captured_statements() {
        #[allow(clippy::type_complexity)]
        let cases: Vec<(&str, &str, Result<(&str, &str), (&str, &str)>)> = vec![
            (
                "\"person\".\"name\" = :name1a2b",
                r#"{"name1a2b":"acme"}"#,
                Ok(("\"person\".\"name\" = $1", r#"["acme"]"#)),
            ),
            (
                "\"id\" IN (:...ids)",
                r#"{"ids":["a","b","c"]}"#,
                Ok(("\"id\" IN ($1, $2, $3)", r#"["a","b","c"]"#)),
            ),
            (
                "\"a\" = :value OR \"b\" = :value",
                r#"{"value":42}"#,
                Ok(("\"a\" = $1 OR \"b\" = $1", r#"[42]"#)),
            ),
            (
                "\"createdAt\" < :cutoff::timestamptz + interval '1 millisecond'",
                r#"{"cutoff":"2026-01-01"}"#,
                Ok((
                    "\"createdAt\" < $1::timestamptz + interval '1 millisecond'",
                    r#"["2026-01-01"]"#,
                )),
            ),
            (
                "\"label\" = 'a:b' AND \"id\" = :id",
                r#"{"id":7}"#,
                Ok(("\"label\" = 'a:b' AND \"id\" = $1", r#"[7]"#)),
            ),
            (
                "\"weird:column\" = :id",
                r#"{"id":1}"#,
                Ok(("\"weird:column\" = $1", r#"[1]"#)),
            ),
            (
                "\"note\" = 'it''s here' AND \"id\" = :id",
                r#"{"id":1}"#,
                Ok(("\"note\" = 'it''s here' AND \"id\" = $1", r#"[1]"#)),
            ),
            (
                "\"id\" = :missing",
                r#"{}"#,
                Err((
                    "MISSING_PARAMETER",
                    "Parameter \":missing\" is referenced by the query but was never provided",
                )),
            ),
            (
                "\"id\" IN (:...ids)",
                r#"{"ids":[]}"#,
                Ok(("\"id\" IN (NULL)", r#"[]"#)),
            ),
            (
                "\"a\" IN (:...ids) OR \"z\" = :ids__0",
                r#"{"ids":["a","b"],"ids__0":99}"#,
                Ok(("\"a\" IN ($1, $2) OR \"z\" = $3", r#"["a","b",99]"#)),
            ),
            (
                "\"id\" IN (:...ids)",
                r#"{"ids":"a"}"#,
                Err((
                    "INVALID_PARAMETER",
                    "Parameter \":...ids\" expects an array",
                )),
            ),
            (
                "\"a\" = 'unterminated",
                r#"{}"#,
                Err(("MALFORMED_SQL", "Unterminated string literal in SQL")),
            ),
            (
                "\"a\" = :value",
                r#"{"value":":notAParameter"}"#,
                Ok(("\"a\" = $1", r#"[":notAParameter"]"#)),
            ),
        ];

        for (sql, params, expected) in cases {
            let parameters: Map<String, Value> = serde_json::from_str(params).unwrap();
            let result = compile_named_parameters(sql, &parameters);

            match expected {
                Ok((text, values)) => {
                    let statement =
                        result.unwrap_or_else(|error| panic!("{sql} failed: {}", error.message()));
                    assert_eq!(statement.text, text, "{sql}");
                    let values: Value = serde_json::from_str(values).unwrap();
                    assert_eq!(Value::Array(statement.values), values, "{sql} values");
                }
                Err((code, message)) => {
                    let error = result.err().unwrap_or_else(|| panic!("{sql} must fail"));
                    assert_eq!(error.code(), code, "{sql}");
                    assert_eq!(error.message(), message, "{sql}");
                }
            }
        }
    }

    #[test]
    fn a_repeated_parameter_reuses_its_placeholder() {
        let parameters = Map::from_iter([("value".to_string(), json!(42))]);
        let statement =
            compile_named_parameters("\"a\" = :value AND \"b\" = :value", &parameters).unwrap();

        assert_eq!(statement.text, "\"a\" = $1 AND \"b\" = $1");
        assert_eq!(statement.values, vec![json!(42)]);
    }

    #[test]
    fn the_in_spread_becomes_one_placeholder_per_item() {
        let parameters = Map::from_iter([("ids".to_string(), json!(["a", "b", "c"]))]);
        let statement = compile_named_parameters("\"id\" IN (:...ids)", &parameters).unwrap();

        assert_eq!(statement.text, "\"id\" IN ($1, $2, $3)");
        assert_eq!(statement.values, vec![json!("a"), json!("b"), json!("c")]);
    }

    #[test]
    fn a_compile_of_a_fragment_pair_binds_once_per_distinct_value() {
        let parameters: Map<String, Value> = Map::from_iter([
            ("jobTitle0000000000".to_string(), json!("")),
            ("name10000000000".to_string(), json!(["Ada", "Bob"])),
        ]);
        let sql = "\"person\".\"jobTitle\" != :jobTitle0000000000 \
                   AND \"person\".\"name\" IN (:...name10000000000) \
                   AND \"person\".\"jobTitle\" IS NOT NULL";
        let statement = compile_named_parameters(sql, &parameters).unwrap();

        assert_eq!(
            statement.text,
            "\"person\".\"jobTitle\" != $1 AND \"person\".\"name\" IN ($2, $3) AND \"person\".\"jobTitle\" IS NOT NULL"
        );
        assert_eq!(
            statement.values,
            vec![json!(""), json!("Ada"), json!("Bob")]
        );
    }

    #[test]
    fn cast_and_quoted_text_never_become_parameters() {
        let parameters = Map::from_iter([("cutoff".to_string(), json!("2026-01-01"))]);
        let sql = "\"createdAt\" < :cutoff::timestamptz + interval '1 millisecond'";
        let statement = compile_named_parameters(sql, &parameters).unwrap();

        assert_eq!(
            statement.text,
            "\"createdAt\" < $1::timestamptz + interval '1 millisecond'"
        );
    }

    #[test]
    fn an_unterminated_literal_is_malformed_sql() {
        let error = compile_named_parameters("\"a\" = 'unterminated", &Map::new()).unwrap_err();
        assert_eq!(error.code(), "MALFORMED_SQL");
        assert_eq!(error.message(), "Unterminated string literal in SQL");

        let error = compile_named_parameters("\"a\" = \"unterminated", &Map::new()).unwrap_err();
        assert_eq!(error.message(), "Unterminated quoted identifier in SQL");
    }

    #[test]
    fn a_bare_colon_or_double_colon_is_left_alone() {
        let parameters = Map::from_iter([("a".to_string(), json!(1)), ("b".to_string(), json!(2))]);
        let statement =
            compile_named_parameters("text::text = :a OR other::int = :b", &parameters).unwrap();

        assert_eq!(statement.text, "text::text = $1 OR other::int = $2");
    }
}
