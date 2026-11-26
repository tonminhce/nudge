#![allow(dead_code)]

use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;
use crate::query::filter::model::Operator;

pub const MAX_RELATION_FILTER_DEPTH: usize = 1;

pub const STRING_FILTER_OPERATORS: [Operator; 12] = [
    Operator::Eq,
    Operator::Neq,
    Operator::Gt,
    Operator::Gte,
    Operator::Lt,
    Operator::Lte,
    Operator::In,
    Operator::Is,
    Operator::Like,
    Operator::ILike,
    Operator::StartsWith,
    Operator::EndsWith,
];

pub const NUMBER_FILTER_OPERATORS: [Operator; 8] = [
    Operator::Eq,
    Operator::Neq,
    Operator::Gt,
    Operator::Gte,
    Operator::Lt,
    Operator::Lte,
    Operator::In,
    Operator::Is,
];

pub const FIELD_REFERENCE_OPERATOR_NAMES: [&str; 4] =
    ["gtField", "gteField", "ltField", "lteField"];

pub const BOOLEAN_FILTER_OPERATORS: [Operator; 2] = [Operator::Eq, Operator::Is];

pub const DATE_FILTER_OPERATORS: [Operator; 8] = NUMBER_FILTER_OPERATORS;

pub const UUID_FILTER_OPERATORS: [Operator; 8] = NUMBER_FILTER_OPERATORS;

pub const ARRAY_FILTER_OPERATORS: [Operator; 3] = [
    Operator::ContainsIlike,
    Operator::Is,
    Operator::IsEmptyArray,
];

pub const MULTI_SELECT_FILTER_OPERATORS: [Operator; 3] =
    [Operator::ContainsAny, Operator::Is, Operator::IsEmptyArray];

pub const ENUM_FILTER_OPERATORS: [Operator; 10] = [
    Operator::Eq,
    Operator::Neq,
    Operator::Gt,
    Operator::Gte,
    Operator::Lt,
    Operator::Lte,
    Operator::In,
    Operator::ContainsAny,
    Operator::Is,
    Operator::IsEmptyArray,
];

pub const RAW_JSON_FILTER_OPERATORS: [Operator; 3] =
    [Operator::Is, Operator::Like, Operator::ILike];

pub const RICH_TEXT_FILTER_OPERATORS: [Operator; 1] = [Operator::ILike];

pub const COMPOSITE_FILTER_OPERATORS: [Operator; 3] = [Operator::Eq, Operator::Neq, Operator::Is];

pub fn operators_for_field_type(field_type: FieldMetadataType) -> Vec<&'static str> {
    let operators: &[Operator] = match field_type {
        FieldMetadataType::Text => &STRING_FILTER_OPERATORS,
        FieldMetadataType::Number | FieldMetadataType::Numeric | FieldMetadataType::Position => {
            &NUMBER_FILTER_OPERATORS
        }
        FieldMetadataType::Boolean => &BOOLEAN_FILTER_OPERATORS,
        FieldMetadataType::Date | FieldMetadataType::DateTime => &DATE_FILTER_OPERATORS,
        FieldMetadataType::Uuid
        | FieldMetadataType::Relation
        | FieldMetadataType::MorphRelation => &UUID_FILTER_OPERATORS,
        FieldMetadataType::Array => &ARRAY_FILTER_OPERATORS,
        FieldMetadataType::MultiSelect => &MULTI_SELECT_FILTER_OPERATORS,
        FieldMetadataType::Select | FieldMetadataType::Rating => &ENUM_FILTER_OPERATORS,
        FieldMetadataType::RawJson | FieldMetadataType::Files => &RAW_JSON_FILTER_OPERATORS,
        FieldMetadataType::RichText => &RICH_TEXT_FILTER_OPERATORS,
        FieldMetadataType::TsVector
        | FieldMetadataType::Actor
        | FieldMetadataType::Address
        | FieldMetadataType::Currency
        | FieldMetadataType::Emails
        | FieldMetadataType::FullName
        | FieldMetadataType::Links
        | FieldMetadataType::Phones => &COMPOSITE_FILTER_OPERATORS,
    };

    let mut names: Vec<&'static str> = operators.iter().map(|operator| operator.as_str()).collect();

    if matches!(
        field_type,
        FieldMetadataType::Number
            | FieldMetadataType::Numeric
            | FieldMetadataType::Position
            | FieldMetadataType::Date
            | FieldMetadataType::DateTime
    ) {
        names.extend(FIELD_REFERENCE_OPERATOR_NAMES);
    }

    names
}

pub fn is_field_reference_operator(operator: &str) -> bool {
    FIELD_REFERENCE_OPERATOR_NAMES.contains(&operator)
}

pub fn is_operator_allowed_for_field_type(
    operator: Operator,
    field_type: FieldMetadataType,
) -> bool {
    operators_for_field_type(field_type).contains(&operator.as_str())
}

pub fn field_reference_comparison(operator: &str) -> Option<&'static str> {
    match operator {
        "gtField" => Some(">"),
        "gteField" => Some(">="),
        "ltField" => Some("<"),
        "lteField" => Some("<="),
        _ => None,
    }
}

pub struct FieldReference {
    pub relation_field_name: Option<String>,
    pub field_name: String,
}

pub fn parse_field_reference_value(value: &Value) -> Option<FieldReference> {
    let raw = value.as_str()?;

    if raw.is_empty() {
        return None;
    }

    let segments: Vec<&str> = raw.split('.').collect();

    match segments.len() {
        1 => Some(FieldReference {
            relation_field_name: None,
            field_name: segments[0].to_string(),
        }),
        2 if !segments[0].is_empty() && !segments[1].is_empty() => Some(FieldReference {
            relation_field_name: Some(segments[0].to_string()),
            field_name: segments[1].to_string(),
        }),
        _ => None,
    }
}

pub fn compute_morph_or_relation_field_join_column_name(field_name: &str) -> String {
    format!("{field_name}Id")
}

pub fn remove_sql_ddl_injection(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

pub fn escape_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

pub fn escape_literal(value: &str) -> String {
    let mut escaped = String::from("'");
    let mut has_backslash = false;

    for character in value.chars() {
        match character {
            '\'' => escaped.push_str("''"),
            '\\' => {
                escaped.push_str("\\\\");
                has_backslash = true;
            }
            other => escaped.push(other),
        }
    }

    escaped.push('\'');

    if has_backslash {
        escaped = format!("E{escaped}");
    }

    escaped
}

pub fn is_safe_ts_vector_expression(expression: &str) -> bool {
    const FORBIDDEN_TOKENS: [&str; 6] = ["\0", ";", "--", "/*", "*/", "$"];

    if FORBIDDEN_TOKENS
        .iter()
        .any(|token| expression.contains(token))
    {
        return false;
    }

    has_balanced_parentheses(expression)
}

pub fn assert_safe_ts_vector_expression(expression: &str) -> bool {
    is_safe_ts_vector_expression(expression)
}

fn has_balanced_parentheses(expression: &str) -> bool {
    #[derive(PartialEq)]
    enum Context {
        Code,
        String,
        Identifier,
    }

    let mut depth = 0i64;
    let mut context = Context::Code;
    let mut characters = expression.chars().peekable();

    while let Some(character) = characters.next() {
        match context {
            Context::String => {
                if character == '\'' {
                    if characters.peek() == Some(&'\'') {
                        characters.next();
                    } else {
                        context = Context::Code;
                    }
                }
            }
            Context::Identifier => {
                if character == '"' {
                    if characters.peek() == Some(&'"') {
                        characters.next();
                    } else {
                        context = Context::Code;
                    }
                }
            }
            Context::Code => match character {
                '\'' => context = Context::String,
                '"' => context = Context::Identifier,
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
    }

    depth == 0 && context == Context::Code
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn string_group_allowlist_excludes_regex_and_includes_like_family() {
        let operators = operators_for_field_type(FieldMetadataType::Text);
        for expected in [
            "eq",
            "neq",
            "gt",
            "gte",
            "lt",
            "lte",
            "in",
            "is",
            "like",
            "ilike",
            "startsWith",
            "endsWith",
        ] {
            assert!(operators.contains(&expected), "missing {expected}");
        }
        for forbidden in ["regex", "iregex", "search", "containsAny", "gtField"] {
            assert!(!operators.contains(&forbidden), "unexpected {forbidden}");
        }
    }

    #[test]
    fn numeric_group_allowlist_carries_field_references() {
        for field_type in [
            FieldMetadataType::Number,
            FieldMetadataType::Numeric,
            FieldMetadataType::Position,
        ] {
            let operators = operators_for_field_type(field_type);
            for expected in [
                "eq", "neq", "gt", "gte", "lt", "lte", "in", "is", "gtField", "gteField",
                "ltField", "lteField",
            ] {
                assert!(
                    operators.contains(&expected),
                    "{field_type:?} missing {expected}"
                );
            }
            assert!(
                !operators.contains(&"like"),
                "{field_type:?} unexpected like"
            );
        }
    }

    #[test]
    fn boolean_group_allowlist_is_eq_and_is_only() {
        assert_eq!(
            operators_for_field_type(FieldMetadataType::Boolean),
            vec!["eq", "is"]
        );
    }

    #[test]
    fn date_groups_share_numeric_plus_field_references() {
        for field_type in [FieldMetadataType::Date, FieldMetadataType::DateTime] {
            let operators = operators_for_field_type(field_type);
            assert!(operators.contains(&"gtField"));
            assert!(operators.contains(&"lteField"));
            assert_eq!(operators.len(), 12);
        }
    }

    #[test]
    fn uuid_group_allowlist_has_no_like_or_field_references() {
        let operators = operators_for_field_type(FieldMetadataType::Uuid);
        assert_eq!(
            operators,
            vec!["eq", "neq", "gt", "gte", "lt", "lte", "in", "is"]
        );
        assert_eq!(
            operators_for_field_type(FieldMetadataType::Relation),
            operators
        );
        assert_eq!(
            operators_for_field_type(FieldMetadataType::MorphRelation),
            operators
        );
    }

    #[test]
    fn array_and_multi_select_groups_use_membership_operators() {
        assert_eq!(
            operators_for_field_type(FieldMetadataType::Array),
            vec!["containsIlike", "is", "isEmptyArray"]
        );
        assert_eq!(
            operators_for_field_type(FieldMetadataType::MultiSelect),
            vec!["containsAny", "is", "isEmptyArray"]
        );
    }

    #[test]
    fn enum_group_adds_contains_any_and_is_empty_array() {
        let operators = operators_for_field_type(FieldMetadataType::Select);
        for expected in [
            "eq",
            "neq",
            "gt",
            "gte",
            "lt",
            "lte",
            "in",
            "containsAny",
            "is",
            "isEmptyArray",
        ] {
            assert!(operators.contains(&expected), "missing {expected}");
        }
        assert_eq!(
            operators_for_field_type(FieldMetadataType::Rating),
            operators
        );
    }

    #[test]
    fn raw_json_rich_text_and_composite_groups_match_upstream() {
        assert_eq!(
            operators_for_field_type(FieldMetadataType::RawJson),
            vec!["is", "like", "ilike"]
        );
        assert_eq!(
            operators_for_field_type(FieldMetadataType::Files),
            vec!["is", "like", "ilike"]
        );
        assert_eq!(
            operators_for_field_type(FieldMetadataType::RichText),
            vec!["ilike"]
        );
        let composite = vec!["eq", "neq", "is"];
        for field_type in [
            FieldMetadataType::TsVector,
            FieldMetadataType::Actor,
            FieldMetadataType::Address,
            FieldMetadataType::Currency,
            FieldMetadataType::Emails,
            FieldMetadataType::FullName,
            FieldMetadataType::Links,
            FieldMetadataType::Phones,
        ] {
            assert_eq!(
                operators_for_field_type(field_type),
                composite,
                "{field_type:?}"
            );
        }
    }

    #[test]
    fn is_operator_allowed_for_field_type_agrees_with_the_allowlist() {
        assert!(is_operator_allowed_for_field_type(
            Operator::EndsWith,
            FieldMetadataType::Text
        ));
        assert!(!is_operator_allowed_for_field_type(
            Operator::EndsWith,
            FieldMetadataType::Uuid
        ));
        assert!(is_operator_allowed_for_field_type(
            Operator::Gt,
            FieldMetadataType::Number
        ));
        assert!(!is_operator_allowed_for_field_type(
            Operator::Search,
            FieldMetadataType::Text
        ));
    }

    #[test]
    fn remove_sql_ddl_injection_strips_non_word_characters() {
        assert_eq!(remove_sql_ddl_injection("my_table"), "my_table");
        assert_eq!(remove_sql_ddl_injection("table\"name"), "tablename");
        assert_eq!(remove_sql_ddl_injection("drop;--"), "drop");
        assert_eq!(remove_sql_ddl_injection("name\" OR 1=1 --"), "nameOR11");
    }

    #[test]
    fn escape_identifier_doubles_inner_quotes() {
        assert_eq!(escape_identifier("myTable"), "\"myTable\"");
        assert_eq!(escape_identifier("my\"table"), "\"my\"\"table\"");
        assert_eq!(escape_identifier("a\"\"b"), "\"a\"\"\"\"b\"");
        assert_eq!(escape_identifier(""), "\"\"");
        assert_eq!(escape_identifier("it's"), "\"it's\"");
        assert_eq!(
            escape_identifier("\"; DROP TABLE users; --"),
            "\"\"\"; DROP TABLE users; --\""
        );
    }

    #[test]
    fn escape_literal_quotes_and_flags_backslashes() {
        assert_eq!(escape_literal("hello"), "'hello'");
        assert_eq!(escape_literal("it's"), "'it''s'");
        assert_eq!(escape_literal("a''b"), "'a''''b'");
        assert_eq!(escape_literal(""), "''");
        assert_eq!(escape_literal("test\\value"), "E'test\\\\value'");
        assert_eq!(escape_literal("it's a \\path"), "E'it''s a \\\\path'");
        assert_eq!(escape_literal("simple"), "'simple'");
        assert_eq!(escape_literal("test\"value"), "'test\"value'");
    }

    #[test]
    fn ts_vector_expression_guard_blocks_injection() {
        let generated =
            "to_tsvector('simple', COALESCE(public.unaccent_immutable(\"name\"), '') || ' ' || COALESCE(\"emailsPrimaryEmail\"::text, ''))";
        assert!(is_safe_ts_vector_expression(generated));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', coalesce(\"name\", ''))) STORED; CREATE TABLE core.\"x\" (a text); --"
        ));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', '') -- comment"
        ));
        assert!(!is_safe_ts_vector_expression(
            "to_tsvector('simple', '') /* comment */"
        ));
        assert!(!is_safe_ts_vector_expression("to_tsvector('simple', $1)"));
        assert!(!assert_safe_ts_vector_expression(
            "to_tsvector('simple', '') ;"
        ));
        assert!(!is_safe_ts_vector_expression("a)b"));
        assert!(!is_safe_ts_vector_expression("a(b"));
        assert!(is_safe_ts_vector_expression("a(b)(c)"));
    }

    #[test]
    fn field_reference_values_parse_same_record_and_relation_paths() {
        let same = parse_field_reference_value(&json!("lastActivityAt")).unwrap();
        assert_eq!(same.relation_field_name, None);
        assert_eq!(same.field_name, "lastActivityAt");

        let through = parse_field_reference_value(&json!("thread.lastActivityAt")).unwrap();
        assert_eq!(through.relation_field_name.as_deref(), Some("thread"));
        assert_eq!(through.field_name, "lastActivityAt");

        for rejected in [
            json!(42),
            json!(""),
            json!("thread."),
            json!("thread.owner.createdAt"),
        ] {
            assert!(
                parse_field_reference_value(&rejected).is_none(),
                "{rejected} must be rejected"
            );
        }
    }

    #[test]
    fn field_reference_operator_names_map_to_sql_comparisons() {
        assert_eq!(field_reference_comparison("gtField"), Some(">"));
        assert_eq!(field_reference_comparison("gteField"), Some(">="));
        assert_eq!(field_reference_comparison("ltField"), Some("<"));
        assert_eq!(field_reference_comparison("lteField"), Some("<="));
        assert_eq!(field_reference_comparison("eq"), None);
        assert!(is_field_reference_operator("lteField"));
        assert!(!is_field_reference_operator("lte"));
    }

    #[test]
    fn relation_join_column_name_is_the_field_name_with_id_suffix() {
        assert_eq!(
            compute_morph_or_relation_field_join_column_name("company"),
            "companyId"
        );
    }
}
