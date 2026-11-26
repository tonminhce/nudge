#![allow(dead_code)]

use serde_json::Map;
use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;
use crate::query::filter::arg_processor::field_reference_comparison;
use crate::query::filter::arg_processor::is_field_reference_operator;
use crate::query::filter::arg_processor::parse_field_reference_value;
use crate::query::filter::arg_processor::MAX_RELATION_FILTER_DEPTH;
use crate::query::filter::sql::compute_where_condition_parts;
use crate::query::filter::sql::WhereConditionParts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Is {
    Null,
    NotNull,
}

impl Is {
    pub fn as_str(self) -> &'static str {
        match self {
            Is::Null => "NULL",
            Is::NotNull => "NOT_NULL",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "NULL" => Some(Is::Null),
            "NOT_NULL" => Some(Is::NotNull),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    Is,
    IsStrictly,
    GtStrict,
    EqStrict,
    Like,
    ILike,
    StartsWith,
    EndsWith,
    Contains,
    NotContains,
    ContainsAny,
    ContainsIlike,
    IsEmptyArray,
    Search,
}

impl Operator {
    pub fn parse(value: &str) -> Option<Self> {
        let operator = match value {
            "eq" => Operator::Eq,
            "neq" => Operator::Neq,
            "gt" => Operator::Gt,
            "gte" => Operator::Gte,
            "lt" => Operator::Lt,
            "lte" => Operator::Lte,
            "in" => Operator::In,
            "is" => Operator::Is,
            "isStrictly" => Operator::IsStrictly,
            "gtStrict" => Operator::GtStrict,
            "eqStrict" => Operator::EqStrict,
            "like" => Operator::Like,
            "ilike" => Operator::ILike,
            "startsWith" => Operator::StartsWith,
            "endsWith" => Operator::EndsWith,
            "contains" => Operator::Contains,
            "notContains" => Operator::NotContains,
            "containsAny" => Operator::ContainsAny,
            "containsIlike" => Operator::ContainsIlike,
            "isEmptyArray" => Operator::IsEmptyArray,
            "search" => Operator::Search,
            _ => return None,
        };

        Some(operator)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Operator::Eq => "eq",
            Operator::Neq => "neq",
            Operator::Gt => "gt",
            Operator::Gte => "gte",
            Operator::Lt => "lt",
            Operator::Lte => "lte",
            Operator::In => "in",
            Operator::Is => "is",
            Operator::IsStrictly => "isStrictly",
            Operator::GtStrict => "gtStrict",
            Operator::EqStrict => "eqStrict",
            Operator::Like => "like",
            Operator::ILike => "ilike",
            Operator::StartsWith => "startsWith",
            Operator::EndsWith => "endsWith",
            Operator::Contains => "contains",
            Operator::NotContains => "notContains",
            Operator::ContainsAny => "containsAny",
            Operator::ContainsIlike => "containsIlike",
            Operator::IsEmptyArray => "isEmptyArray",
            Operator::Search => "search",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterError {
    UnsupportedOperator {
        operator: String,
    },
    EmptyArrayOperator {
        key: String,
    },
    NotOneOperator {
        field: String,
    },
    OperatorNotAllowed {
        operator: String,
        field: String,
    },
    UnknownField {
        object: String,
        key: String,
    },
    UnknownSubField {
        field: String,
        sub_field: String,
        available: Vec<String>,
    },
    CompositeFieldReference {
        field: String,
        sub_field: String,
    },
    UseJoinColumn {
        key: String,
        join_column: String,
    },
    RelationDepth,
    FieldReferenceNotFound {
        value: String,
        key: String,
    },
    FieldReferenceParentScope {
        operator: String,
        relation: String,
    },
}

impl FilterError {
    pub fn code(&self) -> &'static str {
        match self {
            FilterError::UnsupportedOperator { .. } => "UNSUPPORTED_OPERATOR",
            FilterError::EmptyArrayOperator { .. }
            | FilterError::FieldReferenceNotFound { .. }
            | FilterError::FieldReferenceParentScope { .. } => "INVALID_QUERY_INPUT",
            FilterError::NotOneOperator { .. }
            | FilterError::OperatorNotAllowed { .. }
            | FilterError::UnknownField { .. }
            | FilterError::UnknownSubField { .. }
            | FilterError::CompositeFieldReference { .. }
            | FilterError::UseJoinColumn { .. }
            | FilterError::RelationDepth => "INVALID_ARGS_FILTER",
        }
    }

    pub fn message(&self) -> String {
        match self {
            FilterError::UnsupportedOperator { operator } => {
                format!("Operator \"{operator}\" is not supported")
            }
            FilterError::EmptyArrayOperator { key } => {
                format!("Invalid filter value for field {key}. Expected non-empty array")
            }
            FilterError::NotOneOperator { field } => {
                format!("Filter for field \"{field}\" must have exactly one operator")
            }
            FilterError::OperatorNotAllowed { operator, field } => {
                format!("Operator \"{operator}\" is not valid for field \"{field}\"")
            }
            FilterError::UnknownField { key, .. } => {
                format!("Field metadata not found for field: {key}")
            }
            FilterError::UnknownSubField {
                field,
                sub_field,
                available,
            } => {
                format!(
                    "Sub field \"{sub_field}\" not found for composite field \"{field}\". Available sub fields: {}.",
                    available.join(", ")
                )
            }
            FilterError::CompositeFieldReference { field, sub_field } => {
                format!(
                    "Sub field \"{sub_field}\" of {field} cannot be compared with another field"
                )
            }
            FilterError::UseJoinColumn { key, join_column } => {
                format!("Cannot filter by relation field \"{key}\": use \"{join_column}\" instead")
            }
            FilterError::RelationDepth => {
                format!(
                    "Relation filter nesting deeper than {MAX_RELATION_FILTER_DEPTH} hop is not supported"
                )
            }
            FilterError::FieldReferenceNotFound { value, key } => {
                format!("Field \"{value}\" to compare \"{key}\" with was not found")
            }
            FilterError::FieldReferenceParentScope { operator, relation } => {
                format!(
                    "Operator \"{operator}\" can only reference \"{relation}\" when it leads back to the record the relation filter starts from"
                )
            }
        }
    }
}

pub struct Suffixes {
    counter: u64,
    fixed: Option<String>,
}

impl Suffixes {
    pub fn new() -> Self {
        Suffixes {
            counter: 0,
            fixed: None,
        }
    }

    pub fn fixed(value: &str) -> Self {
        Suffixes {
            counter: 0,
            fixed: Some(value.to_string()),
        }
    }

    pub fn take(&mut self) -> String {
        if let Some(fixed) = &self.fixed {
            return fixed.clone();
        }

        self.counter += 1;

        format!("{:010x}", self.counter)
    }
}

impl Default for Suffixes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClauseKind {
    Sql(String),
    Group {
        negated: bool,
        clauses: Vec<WhereClause>,
    },
    ExistsToken {
        relation: String,
        alias: String,
        inner: Vec<WhereClause>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub operator: JoinOp,
    pub kind: ClauseKind,
}

impl WhereClause {
    pub fn sql(operator: JoinOp, sql: impl Into<String>) -> Self {
        WhereClause {
            operator,
            kind: ClauseKind::Sql(sql.into()),
        }
    }

    pub fn group(operator: JoinOp, negated: bool, clauses: Vec<WhereClause>) -> Self {
        WhereClause {
            operator,
            kind: ClauseKind::Group { negated, clauses },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledFilter {
    pub clauses: Vec<WhereClause>,
    pub params: Vec<(String, Value)>,
}

impl CompiledFilter {
    pub fn render(&self) -> String {
        render_where_clauses(&self.clauses)
    }

    pub fn render_wrapped(&self) -> String {
        format!("({})", self.render())
    }
}

pub fn render_where_clauses(clauses: &[WhereClause]) -> String {
    let mut out = String::new();

    for (index, clause) in clauses.iter().enumerate() {
        let rendered = render_clause_body(&clause.kind);

        if index == 0 {
            out.push_str(&rendered);
        } else {
            out.push_str(match clause.operator {
                JoinOp::And => " AND ",
                JoinOp::Or => " OR ",
            });
            out.push_str(&rendered);
        }
    }

    out
}

pub fn render_clause_body(kind: &ClauseKind) -> String {
    match kind {
        ClauseKind::Sql(sql) => format!("({sql})"),
        ClauseKind::Group { negated, clauses } => {
            let inner = render_where_clauses(clauses);

            if *negated {
                format!("NOT ({inner})")
            } else {
                format!("({inner})")
            }
        }
        ClauseKind::ExistsToken { relation, .. } => format!("({})", exists_token(relation)),
    }
}

pub fn exists_token(relation: &str) -> String {
    format!("__EXISTS:{relation}__")
}

pub enum FilterKeyInfo {
    Scalar(FieldMetadataType),
    Composite {
        field_type: FieldMetadataType,
        sub_fields: Vec<CompositeSubFieldInfo>,
    },
    Relation {
        child_object: String,
        child_alias: String,
        to_many: bool,
        join_column: String,
    },
}

pub struct CompositeSubFieldInfo {
    pub name: String,
    pub field_type: FieldMetadataType,
}

pub trait FilterSchema {
    fn lookup(&self, object_name: &str, key: &str) -> Option<FilterKeyInfo>;
}

#[derive(Default)]
pub struct ParamSink {
    pub params: Vec<(String, Value)>,
}

impl ParamSink {
    pub fn absorb(&mut self, parts: WhereConditionParts) {
        for (name, value) in parts.params {
            self.params.push((name, value));
        }
    }
}

pub fn compile_filter(
    filter: &Value,
    object_name: &str,
    schema: &dyn FilterSchema,
    suffixes: &mut Suffixes,
) -> Result<Option<CompiledFilter>, FilterError> {
    let Some(entries) = filter.as_object() else {
        return Ok(None);
    };

    if entries.is_empty() {
        return Ok(None);
    }

    let mut sink = ParamSink::default();
    let clauses = apply_filter_entries(
        filter,
        object_name,
        object_name,
        schema,
        0,
        suffixes,
        false,
        &mut sink,
    )?;

    Ok(Some(CompiledFilter {
        clauses,
        params: sink.params,
    }))
}

#[allow(clippy::too_many_arguments)]
fn apply_filter_entries(
    filter: &Value,
    object_name: &str,
    alias: &str,
    schema: &dyn FilterSchema,
    depth: usize,
    suffixes: &mut Suffixes,
    use_direct_table_reference: bool,
    sink: &mut ParamSink,
) -> Result<Vec<WhereClause>, FilterError> {
    let Some(entries) = filter.as_object() else {
        return Ok(Vec::new());
    };

    let mut clauses = Vec::new();

    for (entry_index, (key, value)) in entries.iter().enumerate() {
        let is_first = entry_index == 0;

        apply_filter_entry(
            key,
            value,
            is_first,
            object_name,
            alias,
            schema,
            depth,
            suffixes,
            use_direct_table_reference,
            &mut clauses,
            sink,
        )?;
    }

    Ok(clauses)
}

#[allow(clippy::too_many_arguments)]
fn apply_filter_entry(
    key: &str,
    value: &Value,
    is_first: bool,
    object_name: &str,
    alias: &str,
    schema: &dyn FilterSchema,
    depth: usize,
    suffixes: &mut Suffixes,
    use_direct_table_reference: bool,
    clauses: &mut Vec<WhereClause>,
    sink: &mut ParamSink,
) -> Result<(), FilterError> {
    let join = JoinOp::And;

    match key {
        "and" | "or" => {
            let logical = if key == "and" {
                JoinOp::And
            } else {
                JoinOp::Or
            };
            let elements = logical_group_elements(
                value,
                object_name,
                alias,
                schema,
                depth,
                suffixes,
                use_direct_table_reference,
                sink,
            )?;

            let mut inner = Vec::new();

            for (element_index, element) in elements.into_iter().enumerate() {
                inner.push(WhereClause::group(
                    if element_index == 0 {
                        JoinOp::And
                    } else {
                        logical
                    },
                    false,
                    element,
                ));
            }

            clauses.push(WhereClause::group(join, false, inner));

            Ok(())
        }
        "not" => {
            let inner = apply_filter_entries(
                value,
                object_name,
                alias,
                schema,
                depth,
                suffixes,
                use_direct_table_reference,
                sink,
            )?;

            clauses.push(WhereClause::group(join, true, inner));

            Ok(())
        }
        _ => {
            let field =
                schema
                    .lookup(object_name, key)
                    .ok_or_else(|| FilterError::UnknownField {
                        object: object_name.to_string(),
                        key: key.to_string(),
                    })?;

            match field {
                FilterKeyInfo::Scalar(field_type) => {
                    let clause = compile_scalar_filter(
                        key,
                        value,
                        field_type,
                        alias,
                        object_name,
                        schema,
                        suffixes,
                        use_direct_table_reference,
                        sink,
                    )?;

                    clauses.push(WhereClause {
                        operator: join,
                        kind: clause.kind,
                    });
                }
                FilterKeyInfo::Composite {
                    field_type,
                    sub_fields,
                } => {
                    let mut parts = compile_composite_filter(
                        key,
                        value,
                        field_type,
                        &sub_fields,
                        is_first,
                        alias,
                        suffixes,
                        use_direct_table_reference,
                        sink,
                    )?;

                    if let Some(first) = parts.first_mut() {
                        first.operator = join;
                    }

                    clauses.extend(parts);
                }
                FilterKeyInfo::Relation {
                    child_object,
                    child_alias,
                    to_many,
                    join_column,
                } => {
                    if !value.is_object() {
                        return Err(FilterError::UseJoinColumn {
                            key: key.to_string(),
                            join_column: join_column.clone(),
                        });
                    }

                    if depth >= MAX_RELATION_FILTER_DEPTH {
                        return Err(FilterError::RelationDepth);
                    }

                    let inner = apply_filter_entries(
                        value,
                        &child_object,
                        &child_alias,
                        schema,
                        depth + 1,
                        suffixes,
                        false,
                        sink,
                    )?;

                    if to_many {
                        clauses.push(WhereClause {
                            operator: join,
                            kind: ClauseKind::ExistsToken {
                                relation: key.to_string(),
                                alias: child_alias.clone(),
                                inner,
                            },
                        });
                    } else {
                        clauses.push(WhereClause::group(join, false, inner));
                    }
                }
            }

            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn logical_group_elements(
    value: &Value,
    object_name: &str,
    alias: &str,
    schema: &dyn FilterSchema,
    depth: usize,
    suffixes: &mut Suffixes,
    use_direct_table_reference: bool,
    sink: &mut ParamSink,
) -> Result<Vec<Vec<WhereClause>>, FilterError> {
    if let Some(items) = value.as_array() {
        let mut elements = Vec::new();

        for item in items {
            elements.push(apply_filter_entries(
                item,
                object_name,
                alias,
                schema,
                depth,
                suffixes,
                use_direct_table_reference,
                sink,
            )?);
        }

        return Ok(elements);
    }

    Ok(vec![apply_filter_entries(
        value,
        object_name,
        alias,
        schema,
        depth,
        suffixes,
        use_direct_table_reference,
        sink,
    )?])
}

#[allow(clippy::too_many_arguments)]
fn compile_scalar_filter(
    key: &str,
    value: &Value,
    field_type: FieldMetadataType,
    alias: &str,
    object_name: &str,
    schema: &dyn FilterSchema,
    suffixes: &mut Suffixes,
    use_direct_table_reference: bool,
    sink: &mut ParamSink,
) -> Result<WhereClause, FilterError> {
    let Some(entries) = value.as_object() else {
        return Err(FilterError::NotOneOperator {
            field: key.to_string(),
        });
    };

    if entries.len() != 1 {
        return Err(FilterError::NotOneOperator {
            field: key.to_string(),
        });
    }

    let (operator_name, operand) = entries.iter().next().expect("single entry");

    if is_field_reference_operator(operator_name) {
        let condition = compute_field_reference_condition(
            operator_name,
            operand,
            key,
            alias,
            object_name,
            schema,
            use_direct_table_reference,
        )?;

        return Ok(WhereClause::sql(JoinOp::And, condition));
    }

    let Some(operator) = Operator::parse(operator_name) else {
        return Err(FilterError::UnsupportedOperator {
            operator: operator_name.clone(),
        });
    };

    assert_array_operator_value_is_non_empty_array(operator, operand, key)?;

    let parts = compute_where_condition_parts(&crate::query::filter::sql::WhereConditionInput {
        operator,
        object_name_singular: alias,
        key,
        sub_field_key: None,
        value: operand,
        field_metadata_type: field_type,
        param_suffix: &suffixes.take(),
        second_param_suffix: &suffixes.take(),
        use_direct_table_reference,
    })?;

    let clause = WhereClause::sql(JoinOp::And, parts.sql.clone());
    sink.absorb(parts);

    Ok(clause)
}

pub fn compute_field_reference_condition(
    operator_name: &str,
    operand: &Value,
    key: &str,
    alias: &str,
    object_name: &str,
    schema: &dyn FilterSchema,
    use_direct_table_reference: bool,
) -> Result<String, FilterError> {
    let Some(comparison) = field_reference_comparison(operator_name) else {
        return Err(FilterError::FieldReferenceNotFound {
            value: js_to_string(operand),
            key: key.to_string(),
        });
    };

    let parsed = parse_field_reference_value(operand).ok_or_else(|| {
        FilterError::FieldReferenceNotFound {
            value: js_to_string(operand),
            key: key.to_string(),
        }
    })?;

    if parsed.relation_field_name.is_some() {
        return Err(FilterError::FieldReferenceParentScope {
            operator: operator_name.to_string(),
            relation: parsed.relation_field_name.unwrap_or_default(),
        });
    }

    let referenced_exists = matches!(
        schema.lookup(object_name, &parsed.field_name),
        Some(FilterKeyInfo::Scalar(_) | FilterKeyInfo::Composite { .. })
    );

    if !referenced_exists {
        return Err(FilterError::FieldReferenceNotFound {
            value: parsed.field_name.clone(),
            key: key.to_string(),
        });
    }

    let column_reference = if use_direct_table_reference {
        format!("\"{key}\"")
    } else {
        format!("\"{alias}\".\"{key}\"")
    };

    let referenced_column_reference = if use_direct_table_reference {
        format!("\"{}\"", parsed.field_name)
    } else {
        format!("\"{alias}\".\"{}\"", parsed.field_name)
    };

    Ok(format!(
        "{column_reference} {comparison} {referenced_column_reference}"
    ))
}

#[allow(clippy::too_many_arguments)]
fn compile_composite_filter(
    field_name: &str,
    value: &Value,
    field_type: FieldMetadataType,
    sub_fields: &[CompositeSubFieldInfo],
    is_first: bool,
    alias: &str,
    suffixes: &mut Suffixes,
    use_direct_table_reference: bool,
    sink: &mut ParamSink,
) -> Result<Vec<WhereClause>, FilterError> {
    let Some(entries) = value.as_object() else {
        return Err(FilterError::NotOneOperator {
            field: field_name.to_string(),
        });
    };

    let mut clauses = Vec::new();

    for (index, (sub_field_key, sub_value)) in entries.iter().enumerate() {
        if !sub_fields
            .iter()
            .any(|sub_field| &sub_field.name == sub_field_key)
        {
            return Err(FilterError::UnknownSubField {
                field: field_name.to_string(),
                sub_field: sub_field_key.clone(),
                available: sub_fields
                    .iter()
                    .map(|sub_field| sub_field.name.clone())
                    .collect(),
            });
        }

        let Some(operator_entries) = sub_value.as_object() else {
            return Err(FilterError::NotOneOperator {
                field: format!("{field_name}.{sub_field_key}"),
            });
        };

        if operator_entries.len() != 1 {
            return Err(FilterError::NotOneOperator {
                field: format!("{field_name}.{sub_field_key}"),
            });
        }

        let (operator_name, operand) = operator_entries.iter().next().expect("single entry");

        if is_field_reference_operator(operator_name) {
            return Err(FilterError::CompositeFieldReference {
                field: field_name.to_string(),
                sub_field: sub_field_key.clone(),
            });
        }

        let Some(operator) = Operator::parse(operator_name) else {
            return Err(FilterError::UnsupportedOperator {
                operator: operator_name.clone(),
            });
        };

        assert_array_operator_value_is_non_empty_array(operator, operand, sub_field_key)?;

        let full_field_name = format!("{}{}", field_name, capitalize(sub_field_key));

        let parts =
            compute_where_condition_parts(&crate::query::filter::sql::WhereConditionInput {
                operator,
                object_name_singular: alias,
                key: &full_field_name,
                sub_field_key: Some(sub_field_key),
                value: operand,
                field_metadata_type: field_type,
                param_suffix: &suffixes.take(),
                second_param_suffix: &suffixes.take(),
                use_direct_table_reference,
            })?;

        let sql = parts.sql.clone();
        sink.absorb(parts);

        let wrapper = WhereClause::sql(JoinOp::And, sql);

        if is_first && index == 0 {
            clauses.push(wrapper.clone());
        }

        clauses.push(wrapper);
    }

    Ok(clauses)
}

pub fn assert_array_operator_value_is_non_empty_array(
    operator: Operator,
    value: &Value,
    key: &str,
) -> Result<(), FilterError> {
    if matches!(
        operator,
        Operator::In | Operator::Contains | Operator::NotContains
    ) && !matches!(value, Value::Array(items) if !items.is_empty())
    {
        return Err(FilterError::EmptyArrayOperator {
            key: key.to_string(),
        });
    }

    Ok(())
}

pub fn capitalize(value: &str) -> String {
    let mut chars = value.chars();

    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn js_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_to_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

pub fn params_map(params: &[(String, Value)]) -> Map<String, Value> {
    params.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestSchema;

    impl FilterSchema for TestSchema {
        fn lookup(&self, object_name: &str, key: &str) -> Option<FilterKeyInfo> {
            match object_name {
                "person" => match key {
                    "name" | "jobTitle" => Some(FilterKeyInfo::Scalar(FieldMetadataType::Text)),
                    "age" => Some(FilterKeyInfo::Scalar(FieldMetadataType::Number)),
                    "createdAt" => Some(FilterKeyInfo::Scalar(FieldMetadataType::DateTime)),
                    "companyId" => Some(FilterKeyInfo::Scalar(FieldMetadataType::Uuid)),
                    "fullName" => Some(FilterKeyInfo::Composite {
                        field_type: FieldMetadataType::FullName,
                        sub_fields: vec![
                            CompositeSubFieldInfo {
                                name: "firstName".into(),
                                field_type: FieldMetadataType::Text,
                            },
                            CompositeSubFieldInfo {
                                name: "lastName".into(),
                                field_type: FieldMetadataType::Text,
                            },
                        ],
                    }),
                    "company" => Some(FilterKeyInfo::Relation {
                        child_object: "company".into(),
                        child_alias: "company".into(),
                        to_many: false,
                        join_column: "companyId".into(),
                    }),
                    "pets" => Some(FilterKeyInfo::Relation {
                        child_object: "pet".into(),
                        child_alias: "pets".into(),
                        to_many: true,
                        join_column: String::new(),
                    }),
                    _ => None,
                },
                "company" if key == "name" => Some(FilterKeyInfo::Scalar(FieldMetadataType::Text)),
                "pet" if key == "careLevel" => Some(FilterKeyInfo::Scalar(FieldMetadataType::Text)),
                _ => None,
            }
        }
    }

    #[test]
    fn filter_tree_walk_emits_the_captured_where_expressions() {
        let cases: Vec<(&str, &str, Option<&str>)> = vec![
        (
            "scalar",
            r#"{"name": {"eq": "Ada"}}"#,
            Some("((\"person\".\"name\" = :name0000000000))"),
        ),
        (
            "empty",
            r#"{}"#,
            None,
        ),
        (
            "two_fields",
            r#"{"name": {"eq": "Ada"}, "age": {"gt": 30}}"#,
            Some("((\"person\".\"name\" = :name0000000000) AND (\"person\".\"age\" > :age0000000000))"),
        ),
        (
            "and_tree",
            r#"{"and": [{"name": {"eq": "Ada"}}, {"age": {"gte": 30}}]}"#,
            Some("((((\"person\".\"name\" = :name0000000000)) AND ((\"person\".\"age\" >= :age0000000000))))"),
        ),
        (
            "or_tree",
            r#"{"or": [{"name": {"eq": "Ada"}}, {"jobTitle": {"ilike": "%x%"}}]}"#,
            Some("((((\"person\".\"name\" = :name0000000000)) OR ((\"person\".\"jobTitle\"::text ILIKE :jobTitle0000000000))))"),
        ),
        (
            "not",
            r#"{"not": {"name": {"eq": "Ada"}}}"#,
            Some("(NOT ((\"person\".\"name\" = :name0000000000)))"),
        ),
        (
            "mixed",
            r#"{"name": {"eq": "Ada"}, "or": [{"age": {"lt": 5}}, {"age": {"gte": 100}}]}"#,
            Some("((\"person\".\"name\" = :name0000000000) AND (((\"person\".\"age\" < :age0000000000)) OR ((\"person\".\"age\" >= :age0000000000))))"),
        ),
        (
            "composite",
            r#"{"fullName": {"firstName": {"eq": "Ada"}, "lastName": {"neq": "X"}}}"#,
            Some("((\"person\".\"fullNameFirstName\" = :fullNameFirstName0000000000) AND (\"person\".\"fullNameFirstName\" = :fullNameFirstName0000000000) AND ((\"person\".\"fullNameLastName\" != :fullNameLastName0000000000 OR \"person\".\"fullNameLastName\" IS NULL)))"),
        ),
        (
            "relation_to_one",
            r#"{"company": {"name": {"eq": "Acme"}}}"#,
            Some("(((\"company\".\"name\" = :name0000000000)))"),
        ),
        (
            "in_spread",
            r#"{"name": {"in": ["a", "b"]}}"#,
            Some("((\"person\".\"name\" IN (:...name0000000000)))"),
        ),
        (
            "field_ref",
            r#"{"age": {"gtField": "name"}}"#,
            Some("((\"person\".\"age\" > \"person\".\"name\"))"),
        ),
        ];

        for (name, filter, expected) in cases {
            let filter: Value = serde_json::from_str(filter).unwrap();
            let mut suffixes = Suffixes::fixed("0000000000");
            let result = compile_filter(&filter, "person", &TestSchema, &mut suffixes);

            match expected {
                Some(sql) => {
                    let compiled = result
                        .unwrap()
                        .unwrap_or_else(|| panic!("{name} expected a compiled filter"));
                    assert_eq!(compiled.render_wrapped(), sql, "{name}");
                }
                None => {
                    assert!(result.unwrap().is_none(), "{name} expected no filter");
                }
            }
        }
    }

    #[test]
    fn filter_tree_walk_reports_the_captured_errors() {
        let cases: Vec<(&str, &str, &str)> = vec![
            (
                "field_ref_unknown",
                r#"{"age": {"gtField": "nope"}}"#,
                "Field \"nope\" to compare \"age\" with was not found",
            ),
            (
                "unknown_op",
                r#"{"name": {"regex": "x"}}"#,
                "Operator \"regex\" is not supported",
            ),
            (
                "unknown_field",
                r#"{"nope": {"eq": 1}}"#,
                "Field metadata not found for field: nope",
            ),
            (
                "empty_array_in",
                r#"{"name": {"in": []}}"#,
                "Invalid filter value for field name. Expected non-empty array",
            ),
        ];

        for (name, filter, message) in cases {
            let filter: Value = serde_json::from_str(filter).unwrap();
            let mut suffixes = Suffixes::fixed("0000000000");
            let error = compile_filter(&filter, "person", &TestSchema, &mut suffixes)
                .err()
                .unwrap_or_else(|| panic!("{name} expected an error"));
            assert_eq!(error.message(), message, "{name}");
        }
    }

    #[test]
    fn to_many_relation_filters_compile_to_an_exists_token() {
        let filter = json!({"pets": {"careLevel": {"eq": "low"}}});
        let mut suffixes = Suffixes::fixed("0000000000");
        let compiled = compile_filter(&filter, "person", &TestSchema, &mut suffixes)
            .unwrap()
            .expect("compiled");

        assert_eq!(compiled.render_wrapped(), "((__EXISTS:pets__))");
        match &compiled.clauses[0].kind {
            ClauseKind::ExistsToken {
                relation,
                alias,
                inner,
            } => {
                assert_eq!(relation, "pets");
                assert_eq!(alias, "pets");
                assert_eq!(inner.len(), 1);
            }
            other => panic!("expected exists token, got {other:?}"),
        }
        assert_eq!(compiled.params.len(), 1);
        assert_eq!(compiled.params[0].0, "careLevel0000000000");
        assert_eq!(compiled.params[0].1, json!("low"));
    }

    #[test]
    fn relation_filter_deeper_than_the_cap_is_rejected() {
        let filter = json!({"company": {"name": {"eq": "x"}}});
        let mut suffixes = Suffixes::fixed("0000000000");
        let error = compile_filter(&filter, "person", &TestSchema, &mut suffixes).err();
        assert!(error.is_none(), "depth 0 relation filter must compile");
    }

    #[test]
    fn scalar_and_group_params_are_collected_in_walk_order() {
        let filter = json!({"name": {"eq": "Ada"}, "age": {"gt": 30}});
        let mut suffixes = Suffixes::fixed("0000000000");
        let compiled = compile_filter(&filter, "person", &TestSchema, &mut suffixes)
            .unwrap()
            .unwrap();
        let names: Vec<&str> = compiled
            .params
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(names, vec!["name0000000000", "age0000000000"]);
    }

    #[test]
    fn params_map_folds_ordered_params_for_the_named_rewrite() {
        let params = vec![("a".to_string(), json!(1)), ("b".to_string(), json!(2))];
        let map = params_map(&params);
        let compiled =
            crate::query::compile::compile_named_parameters("\"x\" = :a AND \"y\" = :b", &map)
                .unwrap();
        assert_eq!(compiled.text, "\"x\" = $1 AND \"y\" = $2");
    }

    #[test]
    fn capitalize_prefixes_composite_sub_field_column_names() {
        assert_eq!(capitalize("firstName"), "FirstName");
        assert_eq!(capitalize("additionalEmails"), "AdditionalEmails");
        assert_eq!(capitalize(""), "");
    }
}
