#![allow(dead_code)]

use super::error::DdlError;

pub fn escape_identifier(identifier: &str) -> Result<String, DdlError> {
    if identifier.contains('\0') {
        return Err(DdlError::null_byte_in_identifier());
    }

    Ok(format!("\"{}\"", identifier.replace('"', "\"\"")))
}

pub fn escape_literal(value: &str) -> Result<String, DdlError> {
    if value.contains('\0') {
        return Err(DdlError::null_byte_in_literal());
    }

    let mut escaped = String::with_capacity(value.len() + 2);
    let mut has_backslash = false;

    escaped.push('\'');

    for character in value.chars() {
        if character == '\'' {
            escaped.push_str("''");
        } else if character == '\\' {
            escaped.push_str("\\\\");
            has_backslash = true;
        } else {
            escaped.push(character);
        }
    }

    escaped.push('\'');

    if has_backslash {
        escaped.insert(0, 'E');
    }

    Ok(escaped)
}

const FORBIDDEN_TS_VECTOR_EXPRESSION_TOKENS: [&str; 6] = ["\0", ";", "--", "/*", "*/", "$"];

fn has_balanced_parentheses(expression: &str) -> bool {
    let chars: Vec<char> = expression.chars().collect();
    let mut depth: i64 = 0;
    let mut context = ExpressionContext::Code;

    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];

        match context {
            ExpressionContext::String => {
                if character == '\'' {
                    if chars.get(index + 1) == Some(&'\'') {
                        index += 1;
                    } else {
                        context = ExpressionContext::Code;
                    }
                }
            }
            ExpressionContext::Identifier => {
                if character == '"' {
                    if chars.get(index + 1) == Some(&'"') {
                        index += 1;
                    } else {
                        context = ExpressionContext::Code;
                    }
                }
            }
            ExpressionContext::Code => match character {
                '\'' => context = ExpressionContext::String,
                '"' => context = ExpressionContext::Identifier,
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                _ => {}
            },
        }

        index += 1;
    }

    depth == 0 && context == ExpressionContext::Code
}

#[derive(Clone, Copy, PartialEq)]
enum ExpressionContext {
    Code,
    String,
    Identifier,
}

pub fn is_safe_ts_vector_expression(expression: &str) -> bool {
    if FORBIDDEN_TS_VECTOR_EXPRESSION_TOKENS
        .iter()
        .any(|token| expression.contains(token))
    {
        return false;
    }

    has_balanced_parentheses(expression)
}

pub fn assert_safe_ts_vector_expression(expression: &str) -> Result<(), DdlError> {
    if !is_safe_ts_vector_expression(expression) {
        return Err(DdlError::unsafe_ts_vector_expression());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_identifier_wraps_and_doubles_quotes() {
        assert_eq!(escape_identifier("person").unwrap(), "\"person\"");
        assert_eq!(escape_identifier("my\"table").unwrap(), "\"my\"\"table\"");
        assert_eq!(escape_identifier("workspace_1").unwrap(), "\"workspace_1\"");
    }

    #[test]
    fn escape_identifier_rejects_null_bytes() {
        let error = escape_identifier("a\0b").unwrap_err();
        assert_eq!(
            error.message,
            "Null bytes are not allowed in PostgreSQL identifiers"
        );
    }

    #[test]
    fn escape_literal_quotes_doubles_and_prefixes_e_on_backslash() {
        assert_eq!(escape_literal("it's").unwrap(), "'it''s'");
        assert_eq!(escape_literal("plain").unwrap(), "'plain'");
        assert_eq!(escape_literal("").unwrap(), "''");
        assert_eq!(escape_literal("back\\slash").unwrap(), "E'back\\\\slash'");
    }

    #[test]
    fn escape_literal_rejects_null_bytes() {
        let error = escape_literal("a\0b").unwrap_err();
        assert_eq!(
            error.message,
            "Null bytes are not allowed in PostgreSQL string literals"
        );
    }

    #[test]
    fn tsvector_expression_safety_rejects_forbidden_tokens_and_unbalanced_parens() {
        assert!(is_safe_ts_vector_expression(
            "to_tsvector('simple', COALESCE(\"a\", ''))"
        ));
        assert!(is_safe_ts_vector_expression(
            "to_tsvector('simple', CASE WHEN 'it''s' THEN \"x\" ELSE \"y\" END)"
        ));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', \"a\"); DROP TABLE x"
        ));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', -- comment"
        ));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', /* comment */ \"a\")"
        ));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', \"a\")$"
        ));
        assert!(!is_safe_ts_vector_expression("to_tsvector('simple', \"a\""));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', \"a\"))"
        ));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', \"unterminated"
        ));
        assert!(is_safe_ts_vector_expression(
            "to_tsvector('simple', \"quoted\"\"ok\")"
        ));
    }

    #[test]
    fn assert_maps_an_unsafe_expression_to_an_error() {
        assert!(assert_safe_ts_vector_expression("to_tsvector('simple', \"a\")").is_ok());
        let error = assert_safe_ts_vector_expression("to_tsvector('simple', \"a\");").unwrap_err();
        assert_eq!(error.code(), "UNSAFE_TS_VECTOR_EXPRESSION");
    }
}
