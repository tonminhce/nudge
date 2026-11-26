#![allow(dead_code)]

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::Map;
use serde_json::Value;

use crate::query::filter::model::Operator;
use crate::query::order::get_effective_scan_order;
use crate::query::order::OrderByDirection;

pub type CursorData = Map<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorError {
    InvalidCursor(String),
    MissingLeafValue { path: String },
}

impl CursorError {
    pub fn code(&self) -> &'static str {
        match self {
            CursorError::InvalidCursor(_) | CursorError::MissingLeafValue { .. } => {
                "INVALID_CURSOR"
            }
        }
    }

    pub fn message(&self) -> String {
        match self {
            CursorError::InvalidCursor(cursor) => format!("Invalid cursor: {cursor}"),
            CursorError::MissingLeafValue { path } => format!(
                "Cursor is missing the value for orderBy field \"{path}\": it was encoded for a different orderBy. Restart pagination without a cursor."
            ),
        }
    }
}

pub fn encode_cursor_data(cursor_data: &CursorData) -> String {
    let payload = serde_json::to_string(&Value::Object(cursor_data.clone())).unwrap_or_default();

    URL_SAFE_NO_PAD.encode(payload.as_bytes())
}

pub fn encode_cursor(cursor_data: &CursorData) -> String {
    encode_cursor_data(cursor_data)
}

pub fn decode_cursor(cursor: &str) -> Result<CursorData, CursorError> {
    let normalized: String = cursor
        .chars()
        .filter_map(|character| match character {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' => Some(character),
            '+' => Some('-'),
            '/' => Some('_'),
            _ => None,
        })
        .collect();

    match URL_SAFE_NO_PAD.decode(&normalized) {
        Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
            Ok(Value::Object(map)) => Ok(map),
            _ => Err(CursorError::InvalidCursor(cursor.to_string())),
        },
        Err(_) => Err(CursorError::InvalidCursor(cursor.to_string())),
    }
}

#[derive(Debug, Default, Clone)]
pub struct PaginationArgs {
    pub after: Option<String>,
    pub before: Option<String>,
}

pub fn get_cursor(args: &PaginationArgs) -> Result<Option<CursorData>, CursorError> {
    if let Some(after) = args.after.as_deref() {
        return decode_cursor(after).map(Some);
    }

    if let Some(before) = args.before.as_deref() {
        return decode_cursor(before).map(Some);
    }

    Ok(None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorPaginationDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPageFlags {
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPage<TItem> {
    pub items: Vec<TItem>,
    pub page_info: CursorPageFlags,
}

pub fn build_cursor_page<TItem>(
    fetched_items: Vec<TItem>,
    limit: usize,
    direction: CursorPaginationDirection,
    has_after_cursor: bool,
    has_before_cursor: bool,
) -> CursorPage<TItem> {
    let has_more_items = fetched_items.len() > limit;
    let mut items = fetched_items.into_iter().take(limit).collect::<Vec<_>>();

    if direction == CursorPaginationDirection::Backward {
        items.reverse();
    }

    CursorPage {
        items,
        page_info: CursorPageFlags {
            has_next_page: if direction == CursorPaginationDirection::Backward {
                has_before_cursor
            } else {
                has_more_items
            },
            has_previous_page: if direction == CursorPaginationDirection::Backward {
                has_more_items
            } else {
                has_after_cursor
            },
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafKind {
    Scalar,
    Composite,
    Relation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderByLeafSpec {
    pub path: Vec<String>,
    pub direction: OrderByDirection,
    pub kind: LeafKind,
    pub can_field_hold_null_value: bool,
}

impl OrderByLeafSpec {
    pub fn scalar(path: &[&str], direction: OrderByDirection, nullable: bool) -> Self {
        OrderByLeafSpec {
            path: path.iter().map(|key| key.to_string()).collect(),
            direction,
            kind: LeafKind::Scalar,
            can_field_hold_null_value: nullable,
        }
    }

    pub fn composite(path: &[&str], direction: OrderByDirection, nullable: bool) -> Self {
        OrderByLeafSpec {
            path: path.iter().map(|key| key.to_string()).collect(),
            direction,
            kind: LeafKind::Composite,
            can_field_hold_null_value: nullable,
        }
    }

    pub fn relation(path: &[&str], direction: OrderByDirection, nullable: bool) -> Self {
        OrderByLeafSpec {
            path: path.iter().map(|key| key.to_string()).collect(),
            direction,
            kind: LeafKind::Relation,
            can_field_hold_null_value: nullable,
        }
    }

    pub fn join_path(&self) -> String {
        self.path.join(".")
    }
}

pub fn get_cursor_value_for_leaf(cursor: &CursorData, leaf: &OrderByLeafSpec) -> Option<Value> {
    let mut current: Option<Value> = Some(Value::Object(cursor.clone()));

    for key in &leaf.path {
        match current {
            Some(Value::Object(ref object)) => {
                current = object.get(key).cloned();
            }
            _ => {
                current = None;
                break;
            }
        }
    }

    if current.is_none() && leaf.path.len() > 1 {
        return cursor.get(&leaf.join_path()).cloned();
    }

    current
}

pub fn get_cursor_value_for_leaf_checked(
    cursor: &CursorData,
    leaf: &OrderByLeafSpec,
) -> Result<Value, CursorError> {
    get_cursor_value_for_leaf(cursor, leaf).ok_or_else(|| CursorError::MissingLeafValue {
        path: leaf.join_path(),
    })
}

fn leaf_operator_filter(leaf: &OrderByLeafSpec, operator: Operator, value: Value) -> Value {
    leaf.path.iter().rev().fold(
        single_entry_object(operator.as_str(), value),
        |nested, key| single_entry_object(key, nested),
    )
}

fn single_entry_map(key: &str, value: Value) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert(key.to_string(), value);

    map
}

fn single_entry_object(key: &str, value: Value) -> Value {
    Value::Object(single_entry_map(key, value))
}

fn or_branch(conditions: Vec<Value>) -> Value {
    single_entry_object("or", Value::Array(conditions))
}

fn and_branch(conditions: Vec<Value>) -> Value {
    single_entry_object("and", Value::Array(conditions))
}

pub fn build_cursor_leaf_where_condition(
    leaf: &OrderByLeafSpec,
    cursor_value: &Value,
    is_forward_pagination: bool,
    is_equality_condition: bool,
) -> Option<Value> {
    let cursor_is_null = cursor_value.is_null();

    if is_equality_condition {
        return Some(if cursor_is_null {
            leaf_operator_filter(
                leaf,
                Operator::IsStrictly,
                Value::String("NULL".to_string()),
            )
        } else {
            leaf_operator_filter(leaf, Operator::EqStrict, cursor_value.clone())
        });
    }

    let scan = get_effective_scan_order(leaf.direction, is_forward_pagination);

    if cursor_is_null {
        return if scan.are_nulls_scanned_last {
            None
        } else {
            Some(leaf_operator_filter(
                leaf,
                Operator::IsStrictly,
                Value::String("NOT_NULL".to_string()),
            ))
        };
    }

    let main_condition = if scan.is_ascending {
        leaf_operator_filter(leaf, Operator::GtStrict, cursor_value.clone())
    } else {
        leaf_operator_filter(leaf, Operator::Lt, cursor_value.clone())
    };

    if scan.are_nulls_scanned_last && leaf.can_field_hold_null_value {
        let null_check = leaf_operator_filter(
            leaf,
            Operator::IsStrictly,
            Value::String("NULL".to_string()),
        );

        return Some(or_branch(vec![main_condition, null_check]));
    }

    Some(main_condition)
}

pub fn compute_cursor_arg_filter(
    cursor: &CursorData,
    leaves: &[OrderByLeafSpec],
    is_forward_pagination: bool,
) -> Result<Vec<Value>, CursorError> {
    let mut leaves_with_cursor_values = Vec::new();

    for leaf in leaves {
        let cursor_value = get_cursor_value_for_leaf_checked(cursor, leaf)?;

        leaves_with_cursor_values.push((leaf, cursor_value));
    }

    let equality_conditions: Vec<Value> = leaves_with_cursor_values
        .iter()
        .filter_map(|(leaf, cursor_value)| {
            build_cursor_leaf_where_condition(leaf, cursor_value, is_forward_pagination, true)
        })
        .collect();

    let mut result: Vec<Value> = Vec::new();

    for (index, (leaf, cursor_value)) in leaves_with_cursor_values.iter().enumerate() {
        let Some(main_condition) =
            build_cursor_leaf_where_condition(leaf, cursor_value, is_forward_pagination, false)
        else {
            continue;
        };

        let mut and_conditions: Vec<Value> =
            equality_conditions.iter().take(index).cloned().collect();
        and_conditions.push(main_condition);

        if and_conditions.len() == 1 {
            result.push(and_conditions.remove(0));
        } else {
            result.push(and_branch(and_conditions));
        }
    }

    Ok(result)
}

pub fn merge_cursor_conditions(conditions: &[Value]) -> Value {
    or_branch(conditions.to_vec())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageInfo {
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

pub fn get_page_info(
    records: &[CursorData],
    flags: &CursorPageFlags,
    encode: &dyn Fn(&CursorData) -> String,
) -> PageInfo {
    PageInfo {
        start_cursor: records.first().map(encode),
        end_cursor: records.last().map(encode),
        has_next_page: flags.has_next_page,
        has_previous_page: flags.has_previous_page,
    }
}

pub fn build_cursor_data(leaf_values: &[(&OrderByLeafSpec, Value)], record_id: &str) -> CursorData {
    let mut data = Map::new();

    for (leaf, value) in leaf_values {
        let mut container = &mut data;

        for key in &leaf.path[..leaf.path.len() - 1] {
            let entry = container
                .entry(key.clone())
                .or_insert_with(|| Value::Object(Map::new()));

            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }

            container = entry.as_object_mut().expect("object inserted");
        }

        container.insert(leaf.path[leaf.path.len() - 1].clone(), value.clone());
    }

    data.insert("id".to_string(), Value::String(record_id.to_string()));

    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serde_json::Value;

    #[test]
    fn cursor_round_trip_matches_the_captured_payloads() {
        let cases: Vec<(&str, &str)> = vec![
            (r#"{"id":"abc"}"#, "eyJpZCI6ImFiYyJ9"),
            (
                r#"{"name":"John","id":"a-b"}"#,
                "eyJuYW1lIjoiSm9obiIsImlkIjoiYS1iIn0",
            ),
            (
                r#"{"fullName.firstName":"Ada","id":"1"}"#,
                "eyJmdWxsTmFtZS5maXJzdE5hbWUiOiJBZGEiLCJpZCI6IjEifQ",
            ),
            (
                r#"{"company":{"name":"Acme"},"id":"9"}"#,
                "eyJjb21wYW55Ijp7Im5hbWUiOiJBY21lIn0sImlkIjoiOSJ9",
            ),
            (
                r#"{"closeDate":null,"id":"u-1"}"#,
                "eyJjbG9zZURhdGUiOm51bGwsImlkIjoidS0xIn0",
            ),
            (
                r#"{"a":1,"b":[1,2],"c":{"d":{"e":true}}}"#,
                "eyJhIjoxLCJiIjpbMSwyXSwiYyI6eyJkIjp7ImUiOnRydWV9fX0",
            ),
            (
                r#"{"id":"ünïcødé-ünïcødé-ünïcødé-ünïcødé01"}"#,
                "eyJpZCI6IsO8bsOvY8O4ZMOpLcO8bsOvY8O4ZMOpLcO8bsOvY8O4ZMOpLcO8bsOvY8O4ZMOpMDEifQ",
            ),
        ];

        for (payload, encoded) in cases {
            let data: CursorData = serde_json::from_str(payload).unwrap();
            assert_eq!(encode_cursor_data(&data), encoded, "{payload}");
            assert_eq!(decode_cursor(encoded).unwrap(), data, "{encoded}");
            assert_eq!(
                decode_cursor(encoded)
                    .map(|d| encode_cursor_data(&d))
                    .unwrap(),
                encoded,
                "{encoded} re-encode"
            );
        }
    }

    #[test]
    fn extra_cursor_round_trips_cover_nesting_and_unicode() {
        let cases: Vec<CursorData> = vec![
            Map::from_iter([("id".to_string(), json!("u1"))]),
            Map::from_iter([("a".to_string(), json!(1)), ("b".to_string(), json!(true))]),
            Map::from_iter([
                ("nested".to_string(), json!({"deep": {"deeper": "x"}})),
                ("id".to_string(), json!("1")),
            ]),
            Map::from_iter([
                ("closeDate".to_string(), Value::Null),
                ("id".to_string(), json!("u2")),
            ]),
            Map::from_iter([
                (
                    "company".to_string(),
                    json!({"contactName": {"firstName": "Ada"}}),
                ),
                ("id".to_string(), json!("u3")),
            ]),
            Map::from_iter([("weird key".to_string(), json!("ünïcødé :*"))]),
            Map::from_iter([
                ("num".to_string(), json!(1.5)),
                ("big".to_string(), json!(2147483647)),
            ]),
            Map::from_iter([("arr".to_string(), json!([1, "two", null]))]),
        ];

        for data in cases {
            let encoded = encode_cursor_data(&data);
            assert!(
                !encoded.contains('-') && !encoded.contains('_') && !encoded.contains('='),
                "base64url no padding: {encoded}"
            );
            assert_eq!(decode_cursor(&encoded).unwrap(), data, "{encoded}");
        }
    }

    #[test]
    fn invalid_cursors_raise_invalid_cursor() {
        for cursor in ["%%%", "not-base64- json::", "", "****"] {
            let error = decode_cursor(cursor).unwrap_err();
            assert_eq!(error.code(), "INVALID_CURSOR", "{cursor:?}");
            assert_eq!(error.message(), format!("Invalid cursor: {cursor}"));
        }

        let truncated = URL_SAFE_NO_PAD.encode(b"{\"id\":");
        let error = decode_cursor(&truncated).unwrap_err();
        assert_eq!(error.code(), "INVALID_CURSOR");
    }

    #[test]
    fn decode_accepts_the_standard_alphabet_and_padding_like_node() {
        let data = Map::from_iter([("id".to_string(), json!("a/b+c"))]);
        let url_safe = encode_cursor_data(&data);
        let standard = url_safe.replace('-', "+").replace('_', "/") + "==";

        assert_eq!(decode_cursor(&url_safe).unwrap(), data);
        assert_eq!(decode_cursor(&standard).unwrap(), data);
    }

    #[test]
    fn get_cursor_prefers_after_then_before() {
        let after = encode_cursor_data(&Map::from_iter([("id".to_string(), json!("after"))]));
        let before = encode_cursor_data(&Map::from_iter([("id".to_string(), json!("before"))]));

        let args = PaginationArgs {
            after: Some(after.clone()),
            before: Some(before.clone()),
        };
        assert_eq!(get_cursor(&args).unwrap().unwrap()["id"], json!("after"));

        let args = PaginationArgs {
            after: None,
            before: Some(before.clone()),
        };
        assert_eq!(get_cursor(&args).unwrap().unwrap()["id"], json!("before"));

        let args = PaginationArgs {
            after: None,
            before: None,
        };
        assert_eq!(get_cursor(&args).unwrap(), None);
    }

    #[test]
    fn build_cursor_page_trims_reverses_and_sets_flags() {
        let page = build_cursor_page(
            vec!["d", "c", "b"],
            2,
            CursorPaginationDirection::Forward,
            false,
            false,
        );
        assert_eq!(page.items, vec!["d", "c"]);
        assert!(page.page_info.has_next_page);
        assert!(!page.page_info.has_previous_page);

        let page = build_cursor_page(
            vec!["b", "c", "d"],
            2,
            CursorPaginationDirection::Backward,
            false,
            true,
        );
        assert_eq!(page.items, vec!["c", "b"]);
        assert!(page.page_info.has_next_page);
        assert!(page.page_info.has_previous_page);

        let page = build_cursor_page(
            vec!["c"],
            2,
            CursorPaginationDirection::Forward,
            true,
            false,
        );
        assert_eq!(page.items, vec!["c"]);
        assert!(!page.page_info.has_next_page);
        assert!(page.page_info.has_previous_page);

        let page = build_cursor_page(
            Vec::<&str>::new(),
            0,
            CursorPaginationDirection::Forward,
            false,
            false,
        );
        assert_eq!(page.items, Vec::<&str>::new());
        assert!(!page.page_info.has_next_page);
        assert!(!page.page_info.has_previous_page);

        let page = build_cursor_page(
            vec!["x", "y"],
            0,
            CursorPaginationDirection::Forward,
            true,
            false,
        );
        assert_eq!(page.items, Vec::<&str>::new());
        assert!(page.page_info.has_next_page);
        assert!(page.page_info.has_previous_page);
    }

    #[test]
    fn build_cursor_page_limit_zero_keeps_the_sentinel_flag() {
        let page = build_cursor_page(
            vec![json!(1)],
            0,
            CursorPaginationDirection::Forward,
            false,
            false,
        );
        assert!(page.items.is_empty());
        assert!(page.page_info.has_next_page);
    }

    #[test]
    fn keyset_conditions_match_the_captured_matrix() {
        #[allow(clippy::type_complexity)]
        let cases: Vec<(&str, &str, Vec<OrderByLeafSpec>, bool, Result<&str, &str>)> = vec![
        (
            "empty",
            r#"{}"#,
            vec![],
            true,
            Ok(r#"[]"#),
        ),
        (
            "fwd_single",
            r#"{"name":"John"}"#,
            vec![OrderByLeafSpec::scalar(&["name"], OrderByDirection::AscNullsLast, true)],
            true,
            Ok(r#"[{"or":[{"name":{"gtStrict":"John"}},{"name":{"isStrictly":"NULL"}}]}]"#),
        ),
        (
            "bwd_single",
            r#"{"name":"John"}"#,
            vec![OrderByLeafSpec::scalar(&["name"], OrderByDirection::AscNullsLast, true)],
            false,
            Ok(r#"[{"name":{"lt":"John"}}]"#),
        ),
        (
            "multi_fwd",
            r#"{"name":"John","age":30}"#,
            vec![OrderByLeafSpec::scalar(&["name"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["age"], OrderByDirection::DescNullsLast, true)],
            true,
            Ok(r#"[{"or":[{"name":{"gtStrict":"John"}},{"name":{"isStrictly":"NULL"}}]},{"and":[{"name":{"eqStrict":"John"}},{"or":[{"age":{"lt":30}},{"age":{"isStrictly":"NULL"}}]}]}]"#),
        ),
        (
            "composite",
            r#"{"fullName":{"firstName":"John","lastName":"Doe"}}"#,
            vec![OrderByLeafSpec::composite(&["fullName", "firstName"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::composite(&["fullName", "lastName"], OrderByDirection::AscNullsLast, true)],
            true,
            Ok(r#"[{"or":[{"fullName":{"firstName":{"gtStrict":"John"}}},{"fullName":{"firstName":{"isStrictly":"NULL"}}}]},{"and":[{"fullName":{"firstName":{"eqStrict":"John"}}},{"or":[{"fullName":{"lastName":{"gtStrict":"Doe"}}},{"fullName":{"lastName":{"isStrictly":"NULL"}}}]}]}]"#),
        ),
        (
            "composite_single",
            r#"{"fullName":{"firstName":"John"}}"#,
            vec![OrderByLeafSpec::composite(&["fullName", "firstName"], OrderByDirection::AscNullsLast, true)],
            true,
            Ok(r#"[{"or":[{"fullName":{"firstName":{"gtStrict":"John"}}},{"fullName":{"firstName":{"isStrictly":"NULL"}}}]}]"#),
        ),
        (
            "composite_dotted",
            r#"{"fullName.firstName":"John","fullName.lastName":"Doe"}"#,
            vec![OrderByLeafSpec::composite(&["fullName", "firstName"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::composite(&["fullName", "lastName"], OrderByDirection::AscNullsLast, true)],
            true,
            Ok(r#"[{"or":[{"fullName":{"firstName":{"gtStrict":"John"}}},{"fullName":{"firstName":{"isStrictly":"NULL"}}}]},{"and":[{"fullName":{"firstName":{"eqStrict":"John"}}},{"or":[{"fullName":{"lastName":{"gtStrict":"Doe"}}},{"fullName":{"lastName":{"isStrictly":"NULL"}}}]}]}]"#),
        ),
        (
            "composite_bwd",
            r#"{"fullName":{"firstName":"John","lastName":"Doe"}}"#,
            vec![OrderByLeafSpec::composite(&["fullName", "firstName"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::composite(&["fullName", "lastName"], OrderByDirection::AscNullsLast, true)],
            false,
            Ok(r#"[{"fullName":{"firstName":{"lt":"John"}}},{"and":[{"fullName":{"firstName":{"eqStrict":"John"}}},{"fullName":{"lastName":{"lt":"Doe"}}}]}]"#),
        ),
        (
            "missing_order_entry",
            r#"{"name":"John"}"#,
            vec![OrderByLeafSpec::scalar(&["age"], OrderByDirection::DescNullsLast, true)],
            true,
            Err("Cursor is missing the value for orderBy field \"age\": it was encoded for a different orderBy. Restart pagination without a cursor."),
        ),
        (
            "null_block_trailing",
            r#"{"closeDate":"2026-01-01T00:00:00Z"}"#,
            vec![OrderByLeafSpec::scalar(&["closeDate"], OrderByDirection::AscNullsLast, true)],
            true,
            Ok(r#"[{"or":[{"closeDate":{"gtStrict":"2026-01-01T00:00:00Z"}},{"closeDate":{"isStrictly":"NULL"}}]}]"#),
        ),
        (
            "null_block_leading",
            r#"{"closeDate":"2026-01-01T00:00:00Z"}"#,
            vec![OrderByLeafSpec::scalar(&["closeDate"], OrderByDirection::AscNullsFirst, true)],
            true,
            Ok(r#"[{"closeDate":{"gtStrict":"2026-01-01T00:00:00Z"}}]"#),
        ),
        (
            "trailing_null_tiebreak",
            r#"{"closeDate":null,"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::scalar(&["closeDate"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Ok(r#"[{"and":[{"closeDate":{"isStrictly":"NULL"}},{"id":{"gtStrict":"uuid-1"}}]}]"#),
        ),
        (
            "leading_null_advance",
            r#"{"closeDate":null,"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::scalar(&["closeDate"], OrderByDirection::AscNullsFirst, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Ok(r#"[{"closeDate":{"isStrictly":"NOT_NULL"}},{"and":[{"closeDate":{"isStrictly":"NULL"}},{"id":{"gtStrict":"uuid-1"}}]}]"#),
        ),
        (
            "backward_swap",
            r#"{"closeDate":"2026-01-01T00:00:00Z"}"#,
            vec![OrderByLeafSpec::scalar(&["closeDate"], OrderByDirection::AscNullsFirst, true)],
            false,
            Ok(r#"[{"or":[{"closeDate":{"lt":"2026-01-01T00:00:00Z"}},{"closeDate":{"isStrictly":"NULL"}}]}]"#),
        ),
        (
            "non_nullable",
            r#"{"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsLast, false)],
            true,
            Ok(r#"[{"id":{"gtStrict":"uuid-1"}}]"#),
        ),
        (
            "missing_scalar",
            r#"{"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::scalar(&["closeDate"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Err("Cursor is missing the value for orderBy field \"closeDate\": it was encoded for a different orderBy. Restart pagination without a cursor."),
        ),
        (
            "missing_composite_prop",
            r#"{"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::composite(&["fullName", "firstName"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Err("Cursor is missing the value for orderBy field \"fullName.firstName\": it was encoded for a different orderBy. Restart pagination without a cursor."),
        ),
        (
            "relation_fwd",
            r#"{"company":{"name":"Acme"},"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::relation(&["company", "name"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Ok(r#"[{"or":[{"company":{"name":{"gtStrict":"Acme"}}},{"company":{"name":{"isStrictly":"NULL"}}}]},{"and":[{"company":{"name":{"eqStrict":"Acme"}}},{"id":{"gtStrict":"uuid-1"}}]}]"#),
        ),
        (
            "relation_null_block",
            r#"{"company":{"name":null},"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::relation(&["company", "name"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Ok(r#"[{"and":[{"company":{"name":{"isStrictly":"NULL"}}},{"id":{"gtStrict":"uuid-1"}}]}]"#),
        ),
        (
            "relation_missing",
            r#"{"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::relation(&["company", "name"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Err("Cursor is missing the value for orderBy field \"company.name\": it was encoded for a different orderBy. Restart pagination without a cursor."),
        ),
        (
            "relation_composite",
            r#"{"company":{"contactName":{"firstName":"Ada","lastName":"Lovelace"}},"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::relation(&["company", "contactName", "firstName"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::relation(&["company", "contactName", "lastName"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Ok(r#"[{"or":[{"company":{"contactName":{"firstName":{"gtStrict":"Ada"}}}},{"company":{"contactName":{"firstName":{"isStrictly":"NULL"}}}}]},{"and":[{"company":{"contactName":{"firstName":{"eqStrict":"Ada"}}}},{"or":[{"company":{"contactName":{"lastName":{"gtStrict":"Lovelace"}}}},{"company":{"contactName":{"lastName":{"isStrictly":"NULL"}}}}]}]},{"and":[{"company":{"contactName":{"firstName":{"eqStrict":"Ada"}}}},{"company":{"contactName":{"lastName":{"eqStrict":"Lovelace"}}}},{"id":{"gtStrict":"uuid-1"}}]}]"#),
        ),
        (
            "relation_composite_missing",
            r#"{"company":{"contactName":{"firstName":"Ada"}},"id":"uuid-1"}"#,
            vec![OrderByLeafSpec::relation(&["company", "contactName", "firstName"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::relation(&["company", "contactName", "lastName"], OrderByDirection::AscNullsLast, true), OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false)],
            true,
            Err("Cursor is missing the value for orderBy field \"company.contactName.lastName\": it was encoded for a different orderBy. Restart pagination without a cursor."),
        ),
        (
            "desc_fwd",
            r#"{"name":"John"}"#,
            vec![OrderByLeafSpec::scalar(&["name"], OrderByDirection::DescNullsLast, true)],
            true,
            Ok(r#"[{"or":[{"name":{"lt":"John"}},{"name":{"isStrictly":"NULL"}}]}]"#),
        ),
        (
            "desc_bwd",
            r#"{"name":"John"}"#,
            vec![OrderByLeafSpec::scalar(&["name"], OrderByDirection::DescNullsLast, true)],
            false,
            Ok(r#"[{"name":{"gtStrict":"John"}}]"#),
        ),
        ];

        for (label, cursor, leaves, is_forward, expected) in cases {
            let cursor: CursorData = serde_json::from_str(cursor).unwrap();

            match expected {
                Ok(result) => {
                    let computed = compute_cursor_arg_filter(&cursor, &leaves, is_forward)
                        .unwrap_or_else(|error| panic!("{label} failed: {}", error.message()));
                    let expected: Value = serde_json::from_str(result).unwrap();
                    assert_eq!(Value::Array(computed), expected, "{label}");
                }
                Err(message) => {
                    let error = compute_cursor_arg_filter(&cursor, &leaves, is_forward)
                        .err()
                        .unwrap_or_else(|| panic!("{label} must fail"));
                    assert_eq!(error.code(), "INVALID_CURSOR", "{label}");
                    assert_eq!(error.message(), message, "{label}");
                }
            }
        }
    }

    #[test]
    fn the_or_id_and_legacy_dotted_forms_resolve_the_same_leaf_value() {
        let leaf = OrderByLeafSpec::composite(
            &["fullName", "firstName"],
            OrderByDirection::AscNullsLast,
            true,
        );
        let nested = Map::from_iter([("fullName".to_string(), json!({"firstName": "Ada"}))]);
        let dotted = Map::from_iter([("fullName.firstName".to_string(), json!("Ada"))]);

        assert_eq!(
            get_cursor_value_for_leaf(&nested, &leaf),
            Some(json!("Ada"))
        );
        assert_eq!(
            get_cursor_value_for_leaf(&dotted, &leaf),
            Some(json!("Ada"))
        );
        assert_eq!(get_cursor_value_for_leaf(&Map::new(), &leaf), None);
    }

    #[test]
    fn equality_and_ordering_conditions_use_strict_operators_only() {
        let leaf = OrderByLeafSpec::scalar(&["name"], OrderByDirection::AscNullsLast, true);

        let equality =
            build_cursor_leaf_where_condition(&leaf, &json!("John"), true, true).unwrap();
        assert_eq!(equality, json!({"name": {"eqStrict": "John"}}));

        let null_equality =
            build_cursor_leaf_where_condition(&leaf, &Value::Null, true, true).unwrap();
        assert_eq!(null_equality, json!({"name": {"isStrictly": "NULL"}}));

        let ordering =
            build_cursor_leaf_where_condition(&leaf, &json!("John"), true, false).unwrap();
        assert_eq!(
            ordering,
            json!({"or": [{"name": {"gtStrict": "John"}}, {"name": {"isStrictly": "NULL"}}]})
        );

        let id_leaf = OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsLast, false);
        let ordering =
            build_cursor_leaf_where_condition(&id_leaf, &json!("u"), true, false).unwrap();
        assert_eq!(ordering, json!({"id": {"gtStrict": "u"}}));
    }

    #[test]
    fn a_cursor_in_a_null_block_continues_on_tiebreakers_only() {
        let leaves = vec![
            OrderByLeafSpec::scalar(&["closeDate"], OrderByDirection::AscNullsLast, true),
            OrderByLeafSpec::scalar(&["id"], OrderByDirection::AscNullsFirst, false),
        ];
        let cursor = Map::from_iter([
            ("closeDate".to_string(), Value::Null),
            ("id".to_string(), json!("uuid-1")),
        ]);
        let conditions = compute_cursor_arg_filter(&cursor, &leaves, true).unwrap();

        assert_eq!(
            Value::Array(conditions),
            json!([{"and": [{"closeDate": {"isStrictly": "NULL"}}, {"id": {"gtStrict": "uuid-1"}}]}])
        );
    }

    #[test]
    fn merged_cursor_conditions_wrap_in_an_or_filter() {
        let merged = merge_cursor_conditions(&[
            json!({"a": {"gtStrict": 1}}),
            json!({"and": [{"a": {"eqStrict": 1}}, {"id": {"gtStrict": 2}}]}),
        ]);

        assert_eq!(
            merged,
            json!({"or": [{"a": {"gtStrict": 1}}, {"and": [{"a": {"eqStrict": 1}}, {"id": {"gtStrict": 2}}]}]})
        );
    }

    #[test]
    fn page_info_encodes_the_first_and_last_record_cursors() {
        let first = Map::from_iter([
            ("name".to_string(), json!("a")),
            ("id".to_string(), json!("1")),
        ]);
        let last = Map::from_iter([
            ("name".to_string(), json!("z")),
            ("id".to_string(), json!("2")),
        ]);
        let flags = CursorPageFlags {
            has_next_page: true,
            has_previous_page: false,
        };
        let info = get_page_info(&[first.clone(), last.clone()], &flags, &encode_cursor_data);

        assert_eq!(info.start_cursor, Some(encode_cursor_data(&first)));
        assert_eq!(info.end_cursor, Some(encode_cursor_data(&last)));
        assert!(info.has_next_page);
        assert!(!info.has_previous_page);

        let empty = get_page_info(&[], &flags, &encode_cursor_data);
        assert_eq!(empty.start_cursor, None);
        assert_eq!(empty.end_cursor, None);
    }

    #[test]
    fn cursor_data_keeps_the_id_last_and_nests_leaf_paths() {
        let name_leaf = OrderByLeafSpec::scalar(&["name"], OrderByDirection::AscNullsLast, true);
        let contact_leaf = OrderByLeafSpec::relation(
            &["company", "contactName", "firstName"],
            OrderByDirection::AscNullsLast,
            true,
        );
        let data = build_cursor_data(
            &[(&name_leaf, json!("Ada")), (&contact_leaf, json!("L"))],
            "id-1",
        );
        let keys: Vec<&String> = data.keys().collect();

        assert_eq!(keys, vec!["name", "company", "id"]);
        assert_eq!(data["company"]["contactName"]["firstName"], json!("L"));
        assert_eq!(data["id"], json!("id-1"));

        let encoded = encode_cursor(&data);
        assert_eq!(decode_cursor(&encoded).unwrap(), data);
    }
}
