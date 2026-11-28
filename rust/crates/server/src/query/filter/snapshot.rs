use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;
use crate::query::filter::arg_processor::is_operator_allowed_for_field_type;
use crate::query::filter::model::Operator;
use crate::query::filter::null_equiv::find_postgres_default_null_equivalent_value;
use crate::query::runner::base::RunnerSchema;

#[derive(Debug, Clone)]
pub enum SnapshotFilterKey {
    Scalar {
        field_type: FieldMetadataType,
        path: Vec<String>,
    },
    Composite {
        sub_fields: Vec<SnapshotCompositeSubField>,
    },
    Relation,
}

#[derive(Debug, Clone)]
pub struct SnapshotCompositeSubField {
    pub name: String,
    pub field_type: FieldMetadataType,
}

pub trait SnapshotFilterSchema {
    fn snapshot_filter_key(&self, object_name: &str, key: &str) -> Option<SnapshotFilterKey>;
}

impl SnapshotFilterSchema for RunnerSchema {
    fn snapshot_filter_key(&self, object_name: &str, key: &str) -> Option<SnapshotFilterKey> {
        let model = self.object(object_name).ok()?;

        if let Some(column) = model.shape.column(key) {
            let path = match &column.composite_parent_field_name {
                Some(parent) => vec![
                    parent.clone(),
                    crate::query::runner::base::lowercase_first(
                        column.column_name.strip_prefix(parent).unwrap_or(key),
                    ),
                ],
                None => vec![column.field_name.clone()],
            };

            return Some(SnapshotFilterKey::Scalar {
                field_type: column.field_metadata_type,
                path,
            });
        }

        if model.shape.relation(key).is_some() {
            return Some(SnapshotFilterKey::Relation);
        }

        let field = model.fields_by_name.get(key)?;

        if crate::workspace_migrations::column_defs::composite_types::is_composite_field_metadata_type(
            crate::workspace_migrations::runner::ddl_field_type(field.field_type),
        ) {
            let sub_fields = model
                .shape
                .columns
                .iter()
                .filter_map(|column| {
                    let parent = column.composite_parent_field_name.as_deref()?;
                    if parent != key {
                        return None;
                    }
                    let suffix = column.column_name.strip_prefix(parent)?;
                    Some(SnapshotCompositeSubField {
                        name: crate::query::runner::base::lowercase_first(suffix),
                        field_type: column.field_metadata_type,
                    })
                })
                .collect();

            return Some(SnapshotFilterKey::Composite { sub_fields });
        }

        Some(SnapshotFilterKey::Scalar {
            field_type: field.field_type,
            path: vec![field.name.clone()],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Evaluation {
    True,
    False,
    Unknown,
    Unsupported,
}

impl Evaluation {
    fn matches(self) -> bool {
        self == Self::True
    }

    fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
            Self::Unsupported => Self::Unsupported,
        }
    }
}

pub fn record_matches_filter(
    record: &Value,
    filter: &Value,
    object_name: &str,
    schema: &dyn SnapshotFilterSchema,
    should_ignore_soft_delete_default_filter: bool,
) -> bool {
    let Some(filter) = filter.as_object() else {
        return false;
    };

    if filter.is_empty() {
        return should_ignore_soft_delete_default_filter || !is_soft_deleted(record);
    }

    if !should_ignore_soft_delete_default_filter && is_soft_deleted(record) {
        return false;
    }

    evaluate_entries(record, filter, object_name, schema).matches()
}

fn is_soft_deleted(record: &Value) -> bool {
    record
        .get("deletedAt")
        .is_some_and(|value| !value.is_null())
}

fn evaluate_entries(
    record: &Value,
    entries: &serde_json::Map<String, Value>,
    object_name: &str,
    schema: &dyn SnapshotFilterSchema,
) -> Evaluation {
    let mut result = Evaluation::True;

    for (key, value) in entries {
        let evaluation = evaluate_entry(record, key, value, object_name, schema);
        if evaluation == Evaluation::Unsupported {
            return evaluation;
        }
        result = combine_and(result, evaluation);
    }

    result
}

fn evaluate_entry(
    record: &Value,
    key: &str,
    value: &Value,
    object_name: &str,
    schema: &dyn SnapshotFilterSchema,
) -> Evaluation {
    match key {
        "and" => evaluate_logical(record, key, value, object_name, schema),
        "or" => evaluate_logical(record, key, value, object_name, schema),
        "not" => {
            let Some(entries) = value.as_object() else {
                return Evaluation::Unsupported;
            };
            if entries.is_empty() {
                return Evaluation::True;
            }
            evaluate_entries(record, entries, object_name, schema).negate()
        }
        _ => {
            let Some(field) = schema.snapshot_filter_key(object_name, key) else {
                return Evaluation::Unsupported;
            };
            let Some(operators) = value.as_object() else {
                return Evaluation::Unsupported;
            };
            if operators.is_empty() {
                return Evaluation::True;
            }

            match field {
                SnapshotFilterKey::Relation => Evaluation::Unsupported,
                SnapshotFilterKey::Scalar { field_type, path } => {
                    let actual = value_at_path(record, &path);
                    evaluate_scalar(actual, &Value::Object(operators.clone()), field_type, None)
                }
                SnapshotFilterKey::Composite { sub_fields } => {
                    let composite = record.get(key);
                    let mut result = Evaluation::True;
                    for (sub_field, condition) in operators {
                        let Some(metadata) = sub_fields
                            .iter()
                            .find(|metadata| metadata.name == *sub_field)
                        else {
                            return Evaluation::Unsupported;
                        };
                        let actual = composite.and_then(|value| value.get(sub_field));
                        let evaluation = evaluate_scalar(
                            actual,
                            condition,
                            metadata.field_type,
                            Some(sub_field),
                        );
                        if evaluation == Evaluation::Unsupported {
                            return evaluation;
                        }
                        result = combine_and(result, evaluation);
                    }
                    result
                }
            }
        }
    }
}

fn evaluate_logical(
    record: &Value,
    key: &str,
    value: &Value,
    object_name: &str,
    schema: &dyn SnapshotFilterSchema,
) -> Evaluation {
    let elements = match value {
        Value::Array(items) => items
            .iter()
            .map(|item| item.as_object().cloned())
            .collect::<Option<Vec<_>>>(),
        Value::Object(entries) => Some(vec![entries.clone()]),
        _ => None,
    };
    let Some(elements) = elements else {
        return Evaluation::Unsupported;
    };
    let mut result = if key == "or" {
        Evaluation::False
    } else {
        Evaluation::True
    };

    for entries in elements {
        let evaluation = if entries.is_empty() {
            Evaluation::True
        } else {
            evaluate_entries(record, &entries, object_name, schema)
        };
        if evaluation == Evaluation::Unsupported {
            return evaluation;
        }
        result = if key == "or" {
            combine_or(result, evaluation)
        } else {
            combine_and(result, evaluation)
        };
    }

    result
}

fn combine_and(left: Evaluation, right: Evaluation) -> Evaluation {
    match (left, right) {
        (Evaluation::Unsupported, _) | (_, Evaluation::Unsupported) => Evaluation::Unsupported,
        (Evaluation::False, _) | (_, Evaluation::False) => Evaluation::False,
        (Evaluation::Unknown, _) | (_, Evaluation::Unknown) => Evaluation::Unknown,
        _ => Evaluation::True,
    }
}

fn combine_or(left: Evaluation, right: Evaluation) -> Evaluation {
    match (left, right) {
        (Evaluation::Unsupported, _) | (_, Evaluation::Unsupported) => Evaluation::Unsupported,
        (Evaluation::True, _) | (_, Evaluation::True) => Evaluation::True,
        (Evaluation::Unknown, _) | (_, Evaluation::Unknown) => Evaluation::Unknown,
        _ => Evaluation::False,
    }
}

fn evaluate_scalar(
    actual: Option<&Value>,
    condition: &Value,
    field_type: FieldMetadataType,
    sub_field: Option<&str>,
) -> Evaluation {
    let Some(condition) = condition.as_object() else {
        return Evaluation::Unsupported;
    };
    if condition.len() != 1 {
        return Evaluation::Unsupported;
    }
    let (name, operand) = condition.iter().next().expect("one operator");
    let Some(operator) = Operator::parse(name) else {
        return Evaluation::Unsupported;
    };
    if !is_operator_allowed_for_field_type(operator, field_type) {
        return Evaluation::Unsupported;
    }

    let null_equivalent =
        find_postgres_default_null_equivalent_value(operand, field_type, sub_field);
    let actual = actual.unwrap_or(&Value::Null);

    match operator {
        Operator::Eq | Operator::Neq => {
            if let Some(equivalent) = null_equivalent {
                let equal = actual.is_null()
                    || actual.as_str() == Some(equivalent.as_str())
                    || (field_type == FieldMetadataType::MultiSelect
                        || field_type == FieldMetadataType::Array)
                        && actual.as_array().is_some_and(Vec::is_empty);
                return Evaluation::from(if operator == Operator::Eq {
                    equal
                } else {
                    !actual.is_null() && !equal
                });
            }
            if operand.is_null() {
                return if operator == Operator::Eq || actual.is_null() {
                    Evaluation::Unknown
                } else {
                    Evaluation::True
                };
            }
            if actual.is_null() {
                return Evaluation::Unknown;
            }
            Evaluation::from(if operator == Operator::Eq {
                values_equal(actual, operand)
            } else {
                !values_equal(actual, operand)
            })
        }
        Operator::Is | Operator::IsStrictly => {
            let is_null = match operand.as_str() {
                Some("NULL") => true,
                Some("NOT_NULL") => false,
                _ => return Evaluation::Unsupported,
            };
            let equivalent = if operator == Operator::Is {
                null_equivalent.as_ref().and_then(|equivalent| {
                    if actual.as_str() == Some(equivalent.as_str())
                        || (field_type == FieldMetadataType::MultiSelect
                            || field_type == FieldMetadataType::Array)
                            && actual.as_array().is_some_and(Vec::is_empty)
                    {
                        Some(true)
                    } else {
                        None
                    }
                })
            } else {
                None
            };
            let is_empty = actual.is_null() || equivalent.unwrap_or(false);
            Evaluation::from(if is_null { is_empty } else { !is_empty })
        }
        Operator::Gt | Operator::Gte | Operator::Lt | Operator::Lte => {
            if actual.is_null() || operand.is_null() {
                return Evaluation::Unknown;
            }
            let Some(ordering) = compare_values(actual, operand) else {
                return Evaluation::Unsupported;
            };
            Evaluation::from(match operator {
                Operator::Gt => ordering.is_gt(),
                Operator::Gte => ordering.is_ge(),
                Operator::Lt => ordering.is_lt(),
                Operator::Lte => ordering.is_le(),
                _ => false,
            })
        }
        Operator::In => {
            let Some(items) = operand.as_array() else {
                return Evaluation::Unsupported;
            };
            if items.is_empty() || actual.is_null() {
                return Evaluation::Unknown;
            }
            Evaluation::from(items.iter().any(|item| values_equal(actual, item)))
        }
        Operator::Like | Operator::ILike => {
            let Some(pattern) = operand.as_str() else {
                return Evaluation::Unsupported;
            };
            let Some(actual) = scalar_text(actual) else {
                return Evaluation::Unknown;
            };
            Evaluation::from(sql_like_matches(
                &actual,
                pattern,
                operator == Operator::ILike,
            ))
        }
        Operator::StartsWith | Operator::EndsWith => {
            let Some(pattern) = operand.as_str() else {
                return Evaluation::Unsupported;
            };
            let Some(actual) = actual.as_str() else {
                return Evaluation::Unknown;
            };
            Evaluation::from(if operator == Operator::StartsWith {
                actual.starts_with(pattern)
            } else {
                actual.ends_with(pattern)
            })
        }
        Operator::ContainsAny => {
            let Some(expected) = operand.as_array() else {
                return Evaluation::Unsupported;
            };
            let Some(actual) = actual.as_array() else {
                return Evaluation::Unknown;
            };
            Evaluation::from(
                expected
                    .iter()
                    .any(|expected| actual.iter().any(|actual| values_equal(actual, expected))),
            )
        }
        Operator::ContainsIlike => {
            let Some(pattern) = operand.as_str() else {
                return Evaluation::Unsupported;
            };
            let Some(actual) = actual.as_array() else {
                return Evaluation::Unknown;
            };
            Evaluation::from(actual.iter().any(|value| {
                value
                    .as_str()
                    .is_some_and(|value| sql_like_matches(value, pattern, true))
            }))
        }
        Operator::IsEmptyArray => {
            let Some(expected) = operand.as_bool() else {
                return Evaluation::Unsupported;
            };
            let empty = actual.is_null() || actual.as_array().is_some_and(Vec::is_empty);
            Evaluation::from(if expected { empty } else { !empty })
        }
        Operator::Contains
        | Operator::NotContains
        | Operator::Search
        | Operator::GtStrict
        | Operator::EqStrict => Evaluation::Unsupported,
    }
}

impl From<bool> for Evaluation {
    fn from(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }
}

fn value_at_path<'a>(record: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut value = record;
    for segment in path {
        value = value.get(segment)?;
    }
    Some(value)
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.as_f64() == right.as_f64(),
        _ => left == right,
    }
}

fn compare_values(left: &Value, right: &Value) -> Option<std::cmp::Ordering> {
    if let (Some(left), Some(right)) = (left.as_f64(), right.as_f64()) {
        return left.partial_cmp(&right);
    }
    left.as_str()?.partial_cmp(right.as_str()?)
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Null => None,
        Value::Array(_) => None,
        Value::Object(_) => Some(String::from("[object Object]")),
    }
}

fn sql_like_matches(value: &str, pattern: &str, case_insensitive: bool) -> bool {
    let value = if case_insensitive {
        value.to_lowercase()
    } else {
        value.to_string()
    };
    let pattern = if case_insensitive {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };
    let value = value.chars().collect::<Vec<_>>();
    let pattern = pattern.chars().collect::<Vec<_>>();
    let mut matched = vec![false; value.len() + 1];
    matched[0] = true;

    for token in pattern {
        let mut next = vec![false; value.len() + 1];
        match token {
            '%' => {
                next[0] = matched[0];
                for index in 1..=value.len() {
                    next[index] = matched[index] || next[index - 1];
                }
            }
            '_' => {
                next[1..(value.len() + 1)].copy_from_slice(&matched[..value.len()]);
            }
            '\\' => continue,
            character => {
                for index in 1..=value.len() {
                    next[index] = matched[index - 1] && value[index - 1] == character;
                }
            }
        }
        matched = next;
    }

    matched[value.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestSchema;

    impl SnapshotFilterSchema for TestSchema {
        fn snapshot_filter_key(&self, object_name: &str, key: &str) -> Option<SnapshotFilterKey> {
            if object_name != "person" {
                return None;
            }
            match key {
                "name" => Some(SnapshotFilterKey::Composite {
                    sub_fields: vec![
                        SnapshotCompositeSubField {
                            name: String::from("firstName"),
                            field_type: FieldMetadataType::Text,
                        },
                        SnapshotCompositeSubField {
                            name: String::from("lastName"),
                            field_type: FieldMetadataType::Text,
                        },
                    ],
                }),
                "age" => Some(SnapshotFilterKey::Scalar {
                    field_type: FieldMetadataType::Number,
                    path: vec![String::from("age")],
                }),
                "deletedAt" => Some(SnapshotFilterKey::Scalar {
                    field_type: FieldMetadataType::DateTime,
                    path: vec![String::from("deletedAt")],
                }),
                "company" => Some(SnapshotFilterKey::Relation),
                _ => None,
            }
        }
    }

    fn matches(record: Value, filter: Value) -> bool {
        record_matches_filter(&record, &filter, "person", &TestSchema, false)
    }

    #[test]
    fn scalar_composite_and_nested_logical_filters_match_snapshots() {
        let record = json!({
            "name": {"firstName": "Ada", "lastName": "Lovelace"},
            "age": 36,
            "deletedAt": null
        });
        assert!(matches(
            record.clone(),
            json!({"name": {"firstName": {"eq": "Ada"}}})
        ));
        assert!(matches(
            record.clone(),
            json!({"and": [
                {"name": {"lastName": {"eq": "Lovelace"}}},
                {"or": [{"age": {"lt": 30}}, {"age": {"gte": 30}}]},
                {"not": {"name": {"firstName": {"eq": "Grace"}}}}
            ]})
        ));
        assert!(!matches(
            record,
            json!({"name": {"firstName": {"eq": "Ada"}, "lastName": {"eq": "Hopper"}}})
        ));
    }

    #[test]
    fn empty_filters_match_only_live_records_unless_deleted_filtering_is_ignored() {
        let live = json!({"deletedAt": null});
        let deleted = json!({"deletedAt": "2026-01-01T00:00:00.000Z"});
        assert!(matches(live.clone(), json!({})));
        assert!(!matches(deleted.clone(), json!({})));
        assert!(record_matches_filter(
            &deleted,
            &json!({}),
            "person",
            &TestSchema,
            true
        ));
    }

    #[test]
    fn explicit_deleted_at_filters_still_apply_to_soft_deletes() {
        let deleted = json!({"deletedAt": "2026-01-01T00:00:00.000Z"});
        assert!(record_matches_filter(
            &deleted,
            &json!({"deletedAt": {"is": "NOT_NULL"}}),
            "person",
            &TestSchema,
            true
        ));
        assert!(!record_matches_filter(
            &deleted,
            &json!({"deletedAt": {"is": "NULL"}}),
            "person",
            &TestSchema,
            true
        ));
    }

    #[test]
    fn unsupported_relation_and_malformed_filters_fail_closed() {
        let record = json!({"name": {"firstName": "Ada"}, "company": "company-1"});
        assert!(!matches(
            record.clone(),
            json!({"company": {"name": {"eq": "Acme"}}})
        ));
        assert!(!matches(
            record.clone(),
            json!({"or": [{"age": {"eq": 36}}, {"company": {"id": {"eq": "company-1"}}}]})
        ));
        assert!(!matches(record, json!({"unknown": {"eq": "x"}})));
    }

    #[test]
    fn text_comparisons_honor_null_equivalents_and_wildcards() {
        let record = json!({"name": {"firstName": "", "lastName": "Lovelace"}});
        assert!(matches(
            record.clone(),
            json!({"name": {"firstName": {"eq": null}}})
        ));
        assert!(matches(
            record.clone(),
            json!({"name": {"lastName": {"ilike": "%love%"}}})
        ));
        assert!(!matches(
            record,
            json!({"name": {"firstName": {"neq": ""}}})
        ));
    }
}
