#![allow(dead_code)]

use serde_json::Map;
use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;
use crate::query::filter::arg_processor::escape_identifier;
use crate::query::filter::arg_processor::remove_sql_ddl_injection;
use crate::query::filter::model::FilterError;
use crate::query::filter::model::Operator;
use crate::query::filter::null_equiv::find_postgres_default_null_equivalent_value;

pub struct WhereConditionParts {
    pub sql: String,
    pub params: Map<String, Value>,
}

pub struct WhereConditionInput<'a> {
    pub operator: Operator,
    pub object_name_singular: &'a str,
    pub key: &'a str,
    pub sub_field_key: Option<&'a str>,
    pub value: &'a Value,
    pub field_metadata_type: FieldMetadataType,
    pub param_suffix: &'a str,
    pub second_param_suffix: &'a str,
    pub use_direct_table_reference: bool,
    pub cast_uuid_parameters: bool,
}

const DATE_TIME_INTERVAL: &str = "::timestamptz + interval '1 millisecond'";

pub fn compute_where_condition_parts(
    input: &WhereConditionInput<'_>,
) -> Result<WhereConditionParts, FilterError> {
    let is_uuid = matches!(
        input.field_metadata_type,
        FieldMetadataType::Uuid | FieldMetadataType::Relation | FieldMetadataType::MorphRelation
    );
    let uuid_cast = if input.cast_uuid_parameters {
        "::uuid"
    } else {
        ""
    };
    let parameter_key = remove_sql_ddl_injection(input.key);
    let ref_str = field_reference(
        input.object_name_singular,
        input.key,
        input.use_direct_table_reference,
    );
    let is_date_time = input.field_metadata_type == FieldMetadataType::DateTime;
    let null_equivalent = find_postgres_default_null_equivalent_value(
        input.value,
        input.field_metadata_type,
        input.sub_field_key,
    );
    let widened = null_equivalent.is_some();
    let param_name = format!("{parameter_key}{}", input.param_suffix);
    let second_param_name = format!("{parameter_key}{}", input.second_param_suffix);
    let widen_suffix = if widened {
        format!(" OR {ref_str} IS NULL")
    } else {
        String::new()
    };

    match input.operator {
        Operator::IsEmptyArray => {
            if input.value == &Value::Bool(true) {
                return Ok(parts(
                    format!("({ref_str} = '{{}}' OR {ref_str} IS NULL)"),
                    Map::new(),
                ));
            }

            Ok(parts(
                format!("({ref_str} IS NOT NULL AND {ref_str} != '{{}}')"),
                Map::new(),
            ))
        }
        Operator::Eq => {
            if is_date_time {
                return Ok(one_param(
                    format!(
                        "({ref_str} >= :{param_name} AND {ref_str} < :{param_name}{DATE_TIME_INTERVAL}){widen_suffix}"
                    ),
                    &param_name,
                    input.value.clone(),
                ));
            }

            Ok(one_param(
                format!("{ref_str} = :{param_name}{uuid_cast}{widen_suffix}"),
                &param_name,
                input.value.clone(),
            ))
        }
        Operator::Neq => {
            if let Some(equivalent) = null_equivalent.as_ref() {
                return Ok(one_param(
                    format!("({ref_str} != :{param_name}{uuid_cast} AND {ref_str} IS NOT NULL)"),
                    &param_name,
                    Value::String(equivalent.clone()),
                ));
            }

            if input.value == &Value::Null {
                return Ok(parts(format!("{ref_str} IS NOT NULL"), Map::new()));
            }

            if is_date_time {
                return Ok(one_param(
                    format!(
                        "({ref_str} < :{param_name} OR {ref_str} >= :{param_name}{DATE_TIME_INTERVAL} OR {ref_str} IS NULL)"
                    ),
                    &param_name,
                    input.value.clone(),
                ));
            }

            Ok(one_param(
                format!("({ref_str} != :{param_name}{uuid_cast} OR {ref_str} IS NULL)"),
                &param_name,
                input.value.clone(),
            ))
        }
        Operator::Gt => {
            if is_date_time {
                return Ok(one_param(
                    format!("{ref_str} >= :{param_name}{DATE_TIME_INTERVAL}"),
                    &param_name,
                    input.value.clone(),
                ));
            }

            Ok(one_param(
                format!("{ref_str} > :{param_name}{uuid_cast}"),
                &param_name,
                input.value.clone(),
            ))
        }
        Operator::Gte => Ok(one_param(
            format!("{ref_str} >= :{param_name}{uuid_cast}"),
            &param_name,
            input.value.clone(),
        )),
        Operator::Lt => Ok(one_param(
            format!("{ref_str} < :{param_name}{uuid_cast}"),
            &param_name,
            input.value.clone(),
        )),
        Operator::Lte => {
            if is_date_time {
                return Ok(one_param(
                    format!("{ref_str} < :{param_name}{DATE_TIME_INTERVAL}"),
                    &param_name,
                    input.value.clone(),
                ));
            }

            Ok(one_param(
                format!("{ref_str} <= :{param_name}{uuid_cast}"),
                &param_name,
                input.value.clone(),
            ))
        }
        Operator::In => {
            if is_uuid {
                return Ok(one_param(
                    format!("{ref_str} = ANY(ARRAY[:...{param_name}]::uuid[])"),
                    &param_name,
                    input.value.clone(),
                ));
            }

            Ok(one_param(
                format!("{ref_str} IN (:...{param_name})"),
                &param_name,
                input.value.clone(),
            ))
        }
        Operator::Is => {
            let null_str = input.value.as_str() == Some("NULL");
            let predicate = if null_str { "NULL" } else { "NOT NULL" };
            let is_widen = if widened {
                format!(" OR {ref_str} = :{second_param_name}")
            } else {
                String::new()
            };
            let sql = format!("{ref_str} IS {predicate}{is_widen}");

            let params = match null_equivalent.as_ref() {
                Some(equivalent) => {
                    let mut map = Map::new();
                    map.insert(second_param_name, Value::String(equivalent.clone()));

                    map
                }
                None => Map::new(),
            };

            let _ = is_uuid;

            Ok(WhereConditionParts { sql, params })
        }
        Operator::IsStrictly => {
            let predicate = if input.value.as_str() == Some("NULL") {
                "NULL"
            } else {
                "NOT NULL"
            };

            Ok(parts(format!("{ref_str} IS {predicate}"), Map::new()))
        }
        Operator::GtStrict => Ok(one_param(
            format!("{ref_str} > :{param_name}{uuid_cast}"),
            &param_name,
            input.value.clone(),
        )),
        Operator::EqStrict => Ok(one_param(
            format!("{ref_str} = :{param_name}{uuid_cast}"),
            &param_name,
            input.value.clone(),
        )),
        Operator::Like => Ok(one_param(
            format!("{ref_str}::text LIKE :{param_name}{widen_suffix}"),
            &param_name,
            js_string(input.value),
        )),
        Operator::ILike => Ok(one_param(
            format!("{ref_str}::text ILIKE :{param_name}{widen_suffix}"),
            &param_name,
            js_string(input.value),
        )),
        Operator::StartsWith => Ok(one_param(
            format!("{ref_str}::text ^@ :{param_name}"),
            &param_name,
            js_string(input.value),
        )),
        Operator::EndsWith => Ok(one_param(
            format!("RIGHT({ref_str}::text, LENGTH(:{param_name})) = :{param_name}"),
            &param_name,
            js_string(input.value),
        )),
        Operator::Contains => Ok(one_param(
            format!("{ref_str} @> ARRAY[:...{param_name}]"),
            &param_name,
            input.value.clone(),
        )),
        Operator::Search => {
            let ts_query = format_search_terms(input.value.as_str().unwrap_or(""), "and");
            let sql = format!(
                "(\n          {ref_str} @@ to_tsquery('simple', public.unaccent_immutable(:{param_name}Ts)) OR\n          public.unaccent_immutable({ref_str}::text) ILIKE public.unaccent_immutable(:{param_name}Like)\n        )"
            );
            let mut params = Map::new();
            let ts_name = format!("{param_name}Ts");
            let like_name = format!("{param_name}Like");
            params.insert(ts_name, Value::String(ts_query));
            params.insert(
                like_name,
                Value::String(format!("%{}%", input.value.as_str().unwrap_or(""))),
            );

            Ok(WhereConditionParts { sql, params })
        }
        Operator::NotContains => Ok(one_param(
            format!("NOT ({ref_str}::text[] && ARRAY[:...{param_name}]::text[])"),
            &param_name,
            input.value.clone(),
        )),
        Operator::ContainsAny => Ok(one_param(
            format!("{ref_str}::text[] && ARRAY[:...{param_name}]::text[]"),
            &param_name,
            input.value.clone(),
        )),
        Operator::ContainsIlike => Ok(one_param(
            format!(
                "EXISTS (SELECT 1 FROM unnest({ref_str}) AS elem WHERE elem ILIKE :{param_name})"
            ),
            &param_name,
            input.value.clone(),
        )),
    }
}

fn field_reference(
    object_name_singular: &str,
    key: &str,
    use_direct_table_reference: bool,
) -> String {
    if use_direct_table_reference {
        escape_identifier(key)
    } else {
        format!(
            "{}.{}",
            escape_identifier(object_name_singular),
            escape_identifier(key)
        )
    }
}

fn parts(sql: String, params: Map<String, Value>) -> WhereConditionParts {
    WhereConditionParts { sql, params }
}

fn one_param(sql: String, name: &str, value: Value) -> WhereConditionParts {
    let mut params = Map::new();
    params.insert(name.to_string(), value);

    WhereConditionParts { sql, params }
}

fn js_string(value: &Value) -> Value {
    match value {
        Value::Null => Value::String("null".to_string()),
        Value::Bool(b) => Value::String(b.to_string()),
        Value::Number(n) => Value::String(n.to_string()),
        Value::String(s) => Value::String(s.clone()),
        Value::Array(items) => Value::String(
            items
                .iter()
                .map(js_string)
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => Value::String("[object Object]".to_string()),
    }
}

pub fn format_search_terms(search_term: &str, operator: &str) -> String {
    let trimmed = search_term.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    let joiner = if operator == "and" { " & " } else { " | " };

    trimmed
        .split_whitespace()
        .map(|word| {
            let escaped = escape_word(word);
            format!("{escaped}:*")
        })
        .collect::<Vec<_>>()
        .join(joiner)
}

fn escape_word(word: &str) -> String {
    let mut out = String::with_capacity(word.len());

    for character in word.chars() {
        if "\\:'&|!()@<>".contains(character) {
            out.push('\\');
        }

        out.push(character);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serde_json::Map;
    use serde_json::Value;

    const SUFFIX: &str = "0000000000";

    fn run(
        operator: Operator,
        object: &str,
        key: &str,
        sub: Option<&str>,
        value: &Value,
        field_type: FieldMetadataType,
        use_direct: bool,
    ) -> Result<WhereConditionParts, FilterError> {
        compute_where_condition_parts(&WhereConditionInput {
            operator,
            object_name_singular: object,
            key,
            sub_field_key: sub,
            value,
            field_metadata_type: field_type,
            param_suffix: SUFFIX,
            second_param_suffix: SUFFIX,
            use_direct_table_reference: use_direct,
            cast_uuid_parameters: false,
        })
    }

    #[test]
    fn every_operator_emits_the_captured_sql_fragment() {
        #[allow(clippy::type_complexity)]
        let cases: Vec<(&str, Operator, &str, &str, Option<&str>, &str, FieldMetadataType, bool, &str, &str)> = vec![
        (
            "is_empty_array_true",
            Operator::IsEmptyArray,
            "pet",
            "traits",
            None,
            "true",
            FieldMetadataType::Array,
            false,
            "(\"pet\".\"traits\" = '{}' OR \"pet\".\"traits\" IS NULL)",
            "{}",
        ),
        (
            "is_empty_array_false",
            Operator::IsEmptyArray,
            "pet",
            "traits",
            None,
            "false",
            FieldMetadataType::MultiSelect,
            false,
            "(\"pet\".\"traits\" IS NOT NULL AND \"pet\".\"traits\" != '{}')",
            "{}",
        ),
        (
            "eq_datetime",
            Operator::Eq,
            "person",
            "createdAt",
            None,
            "\"2020-01-01T00:00:00.123Z\"",
            FieldMetadataType::DateTime,
            false,
            "(\"person\".\"createdAt\" >= :createdAt0000000000 AND \"person\".\"createdAt\" < :createdAt0000000000::timestamptz + interval '1 millisecond')",
            "{\"createdAt0000000000\":\"2020-01-01T00:00:00.123Z\"}",
        ),
        (
            "eq_text",
            Operator::Eq,
            "person",
            "name",
            None,
            "\"Ada\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\" = :name0000000000",
            "{\"name0000000000\":\"Ada\"}",
        ),
        (
            "eq_text_empty",
            Operator::Eq,
            "person",
            "name",
            None,
            "\"\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\" = :name0000000000 OR \"person\".\"name\" IS NULL",
            "{\"name0000000000\":\"\"}",
        ),
        (
            "eq_text_nullstr",
            Operator::Eq,
            "person",
            "name",
            None,
            "\"NULL\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\" = :name0000000000 OR \"person\".\"name\" IS NULL",
            "{\"name0000000000\":\"NULL\"}",
        ),
        (
            "eq_date",
            Operator::Eq,
            "person",
            "birthday",
            None,
            "\"2020-01-01\"",
            FieldMetadataType::Date,
            false,
            "\"person\".\"birthday\" = :birthday0000000000",
            "{\"birthday0000000000\":\"2020-01-01\"}",
        ),
        (
            "eq_number",
            Operator::Eq,
            "person",
            "age",
            None,
            "30",
            FieldMetadataType::Number,
            false,
            "\"person\".\"age\" = :age0000000000",
            "{\"age0000000000\":30}",
        ),
        (
            "eq_bool",
            Operator::Eq,
            "person",
            "active",
            None,
            "true",
            FieldMetadataType::Boolean,
            false,
            "\"person\".\"active\" = :active0000000000",
            "{\"active0000000000\":true}",
        ),
        (
            "eq_uuid",
            Operator::Eq,
            "person",
            "companyId",
            None,
            "\"20202020-0000-4000-8000-000000000001\"",
            FieldMetadataType::Uuid,
            false,
            "\"person\".\"companyId\" = :companyId0000000000",
            "{\"companyId0000000000\":\"20202020-0000-4000-8000-000000000001\"}",
        ),
        (
            "eq_select",
            Operator::Eq,
            "opportunity",
            "pipelineStage",
            None,
            "\"STAGE_1\"",
            FieldMetadataType::Select,
            false,
            "\"opportunity\".\"pipelineStage\" = :pipelineStage0000000000",
            "{\"pipelineStage0000000000\":\"STAGE_1\"}",
        ),
        (
            "neq_null_equiv",
            Operator::Neq,
            "person",
            "jobTitle",
            None,
            "\"\"",
            FieldMetadataType::Text,
            false,
            "(\"person\".\"jobTitle\" != :jobTitle0000000000 AND \"person\".\"jobTitle\" IS NOT NULL)",
            "{\"jobTitle0000000000\":\"\"}",
        ),
        (
            "neq_value",
            Operator::Neq,
            "person",
            "jobTitle",
            None,
            "\"Engineer\"",
            FieldMetadataType::Text,
            false,
            "(\"person\".\"jobTitle\" != :jobTitle0000000000 OR \"person\".\"jobTitle\" IS NULL)",
            "{\"jobTitle0000000000\":\"Engineer\"}",
        ),
        (
            "neq_null",
            Operator::Neq,
            "person",
            "companyId",
            None,
            "null",
            FieldMetadataType::Uuid,
            false,
            "\"person\".\"companyId\" IS NOT NULL",
            "{}",
        ),
        (
            "neq_datetime",
            Operator::Neq,
            "person",
            "createdAt",
            None,
            "\"2026-01-01T00:00:00.000Z\"",
            FieldMetadataType::DateTime,
            false,
            "(\"person\".\"createdAt\" < :createdAt0000000000 OR \"person\".\"createdAt\" >= :createdAt0000000000::timestamptz + interval '1 millisecond' OR \"person\".\"createdAt\" IS NULL)",
            "{\"createdAt0000000000\":\"2026-01-01T00:00:00.000Z\"}",
        ),
        (
            "neq_uuid_value",
            Operator::Neq,
            "person",
            "companyId",
            None,
            "\"20202020-0000-4000-8000-000000000001\"",
            FieldMetadataType::Uuid,
            false,
            "(\"person\".\"companyId\" != :companyId0000000000 OR \"person\".\"companyId\" IS NULL)",
            "{\"companyId0000000000\":\"20202020-0000-4000-8000-000000000001\"}",
        ),
        (
            "neq_date_value",
            Operator::Neq,
            "person",
            "birthday",
            None,
            "\"2020-01-01\"",
            FieldMetadataType::Date,
            false,
            "(\"person\".\"birthday\" != :birthday0000000000 OR \"person\".\"birthday\" IS NULL)",
            "{\"birthday0000000000\":\"2020-01-01\"}",
        ),
        (
            "neq_number_value",
            Operator::Neq,
            "person",
            "age",
            None,
            "30",
            FieldMetadataType::Number,
            false,
            "(\"person\".\"age\" != :age0000000000 OR \"person\".\"age\" IS NULL)",
            "{\"age0000000000\":30}",
        ),
        (
            "gt_datetime",
            Operator::Gt,
            "person",
            "createdAt",
            None,
            "\"2020-01-01T00:00:00.123Z\"",
            FieldMetadataType::DateTime,
            false,
            "\"person\".\"createdAt\" >= :createdAt0000000000::timestamptz + interval '1 millisecond'",
            "{\"createdAt0000000000\":\"2020-01-01T00:00:00.123Z\"}",
        ),
        (
            "gt_text",
            Operator::Gt,
            "person",
            "name",
            None,
            "\"Ada\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\" > :name0000000000",
            "{\"name0000000000\":\"Ada\"}",
        ),
        (
            "gte_number",
            Operator::Gte,
            "person",
            "age",
            None,
            "30",
            FieldMetadataType::Number,
            false,
            "\"person\".\"age\" >= :age0000000000",
            "{\"age0000000000\":30}",
        ),
        (
            "lt_text",
            Operator::Lt,
            "person",
            "name",
            None,
            "\"Zoe\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\" < :name0000000000",
            "{\"name0000000000\":\"Zoe\"}",
        ),
        (
            "lte_datetime",
            Operator::Lte,
            "person",
            "createdAt",
            None,
            "\"2020-01-01T00:00:00.123Z\"",
            FieldMetadataType::DateTime,
            false,
            "\"person\".\"createdAt\" < :createdAt0000000000::timestamptz + interval '1 millisecond'",
            "{\"createdAt0000000000\":\"2020-01-01T00:00:00.123Z\"}",
        ),
        (
            "lte_number",
            Operator::Lte,
            "person",
            "age",
            None,
            "30",
            FieldMetadataType::Number,
            false,
            "\"person\".\"age\" <= :age0000000000",
            "{\"age0000000000\":30}",
        ),
        (
            "in_text",
            Operator::In,
            "person",
            "name",
            None,
            "[\"Ada\",\"Bob\"]",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\" IN (:...name0000000000)",
            "{\"name0000000000\":[\"Ada\",\"Bob\"]}",
        ),
        (
            "is_null",
            Operator::Is,
            "person",
            "jobTitle",
            None,
            "\"NULL\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"jobTitle\" IS NULL OR \"person\".\"jobTitle\" = :jobTitle0000000000",
            "{\"jobTitle0000000000\":\"\"}",
        ),
        (
            "is_not_null",
            Operator::Is,
            "person",
            "jobTitle",
            None,
            "\"NOT_NULL\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"jobTitle\" IS NOT NULL",
            "{}",
        ),
        (
            "is_strictly_null",
            Operator::IsStrictly,
            "person",
            "name",
            None,
            "\"NULL\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\" IS NULL",
            "{}",
        ),
        (
            "is_strictly_notnull",
            Operator::IsStrictly,
            "person",
            "name",
            None,
            "\"NOT_NULL\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\" IS NOT NULL",
            "{}",
        ),
        (
            "eq_strict",
            Operator::EqStrict,
            "person",
            "createdAt",
            None,
            "\"2020-01-01T00:00:00.123Z\"",
            FieldMetadataType::DateTime,
            false,
            "\"person\".\"createdAt\" = :createdAt0000000000",
            "{\"createdAt0000000000\":\"2020-01-01T00:00:00.123Z\"}",
        ),
        (
            "gt_strict",
            Operator::GtStrict,
            "person",
            "createdAt",
            None,
            "\"2020-01-01T00:00:00.123Z\"",
            FieldMetadataType::DateTime,
            false,
            "\"person\".\"createdAt\" > :createdAt0000000000",
            "{\"createdAt0000000000\":\"2020-01-01T00:00:00.123Z\"}",
        ),
        (
            "like",
            Operator::Like,
            "person",
            "name",
            None,
            "\"%Ad%\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\"::text LIKE :name0000000000",
            "{\"name0000000000\":\"%Ad%\"}",
        ),
        (
            "like_empty",
            Operator::Like,
            "person",
            "name",
            None,
            "\"\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\"::text LIKE :name0000000000 OR \"person\".\"name\" IS NULL",
            "{\"name0000000000\":\"\"}",
        ),
        (
            "ilike",
            Operator::ILike,
            "person",
            "name",
            None,
            "\"%ad%\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\"::text ILIKE :name0000000000",
            "{\"name0000000000\":\"%ad%\"}",
        ),
        (
            "starts_with",
            Operator::StartsWith,
            "person",
            "name",
            None,
            "\"Ad\"",
            FieldMetadataType::Text,
            false,
            "\"person\".\"name\"::text ^@ :name0000000000",
            "{\"name0000000000\":\"Ad\"}",
        ),
        (
            "ends_with",
            Operator::EndsWith,
            "person",
            "name",
            None,
            "\"da\"",
            FieldMetadataType::Text,
            false,
            "RIGHT(\"person\".\"name\"::text, LENGTH(:name0000000000)) = :name0000000000",
            "{\"name0000000000\":\"da\"}",
        ),
        (
            "contains",
            Operator::Contains,
            "pet",
            "tags",
            None,
            "[\"a\",\"b\"]",
            FieldMetadataType::Array,
            false,
            "\"pet\".\"tags\" @> ARRAY[:...tags0000000000]",
            "{\"tags0000000000\":[\"a\",\"b\"]}",
        ),
        (
            "search",
            Operator::Search,
            "person",
            "searchVector",
            None,
            "\"Ada Lovelace\"",
            FieldMetadataType::TsVector,
            false,
            "(\n          \"person\".\"searchVector\" @@ to_tsquery('simple', public.unaccent_immutable(:searchVector0000000000Ts)) OR\n          public.unaccent_immutable(\"person\".\"searchVector\"::text) ILIKE public.unaccent_immutable(:searchVector0000000000Like)\n        )",
            "{\"searchVector0000000000Ts\":\"Ada:* & Lovelace:*\",\"searchVector0000000000Like\":\"%Ada Lovelace%\"}",
        ),
        (
            "not_contains",
            Operator::NotContains,
            "pet",
            "tags",
            None,
            "[\"a\"]",
            FieldMetadataType::Array,
            false,
            "NOT (\"pet\".\"tags\"::text[] && ARRAY[:...tags0000000000]::text[])",
            "{\"tags0000000000\":[\"a\"]}",
        ),
        (
            "contains_any",
            Operator::ContainsAny,
            "pet",
            "traits",
            None,
            "[\"x\",\"y\"]",
            FieldMetadataType::MultiSelect,
            false,
            "\"pet\".\"traits\"::text[] && ARRAY[:...traits0000000000]::text[]",
            "{\"traits0000000000\":[\"x\",\"y\"]}",
        ),
        (
            "contains_ilike",
            Operator::ContainsIlike,
            "pet",
            "tags",
            None,
            "\"ad%\"",
            FieldMetadataType::Array,
            false,
            "EXISTS (SELECT 1 FROM unnest(\"pet\".\"tags\") AS elem WHERE elem ILIKE :tags0000000000)",
            "{\"tags0000000000\":\"ad%\"}",
        ),
        (
            "rawjson_like",
            Operator::Like,
            "person",
            "metadata",
            None,
            "\"%x%\"",
            FieldMetadataType::RawJson,
            false,
            "\"person\".\"metadata\"::text LIKE :metadata0000000000",
            "{\"metadata0000000000\":\"%x%\"}",
        ),
        (
            "richtext_ilike",
            Operator::ILike,
            "note",
            "body",
            None,
            "\"%hi%\"",
            FieldMetadataType::RichText,
            false,
            "\"note\".\"body\"::text ILIKE :body0000000000",
            "{\"body0000000000\":\"%hi%\"}",
        ),
        (
            "composite_actor_name",
            Operator::Eq,
            "person",
            "createdByName",
            Some("name"),
            "\"\"",
            FieldMetadataType::Actor,
            false,
            "\"person\".\"createdByName\" = :createdByName0000000000 OR \"person\".\"createdByName\" IS NULL",
            "{\"createdByName0000000000\":\"\"}",
        ),
        (
            "composite_fullname",
            Operator::Eq,
            "person",
            "nameFirstName",
            Some("firstName"),
            "\"\"",
            FieldMetadataType::FullName,
            false,
            "\"person\".\"nameFirstName\" = :nameFirstName0000000000 OR \"person\".\"nameFirstName\" IS NULL",
            "{\"nameFirstName0000000000\":\"\"}",
        ),
        (
            "composite_emails_add",
            Operator::IsEmptyArray,
            "person",
            "emailsAdditionalEmails",
            Some("additionalEmails"),
            "true",
            FieldMetadataType::Emails,
            false,
            "(\"person\".\"emailsAdditionalEmails\" = '{}' OR \"person\".\"emailsAdditionalEmails\" IS NULL)",
            "{}",
        ),
        (
            "direct_reference",
            Operator::Eq,
            "person",
            "name",
            None,
            "\"Ada\"",
            FieldMetadataType::Text,
            true,
            "\"name\" = :name0000000000",
            "{\"name0000000000\":\"Ada\"}",
        ),
        ];

        for (name, operator, object, key, sub, value, field_type, use_direct, sql, params) in cases
        {
            let value: Value = serde_json::from_str(value).unwrap();
            let expected_params: Value = serde_json::from_str(params).unwrap();
            let parts = run(operator, object, key, sub, &value, field_type, use_direct)
                .unwrap_or_else(|error| panic!("{name} failed: {}", error.message()));
            assert_eq!(parts.sql, sql, "{name}");
            assert_eq!(
                Value::Object(parts.params.clone()),
                expected_params,
                "{name} params"
            );
        }
    }

    #[test]
    fn hostile_key_and_object_stay_quoted_and_sanitized() {
        let parts = run(
            Operator::Eq,
            "pe\"rson",
            "name\" OR 1=1 --",
            None,
            &json!("Ada"),
            FieldMetadataType::Text,
            false,
        )
        .unwrap();

        assert_eq!(
            parts.sql,
            "\"pe\"\"rson\".\"name\"\" OR 1=1 --\" = :nameOR110000000000"
        );
        let mut expected = Map::new();
        expected.insert("nameOR110000000000".to_string(), json!("Ada"));
        assert_eq!(parts.params, expected);
    }

    #[test]
    fn null_equivalence_widening_cases() {
        assert_eq!(
            run(
                Operator::Eq,
                "person",
                "name",
                None,
                &json!(""),
                FieldMetadataType::Text,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"name\" = :name0000000000 OR \"person\".\"name\" IS NULL"
        );
        let parts = run(
            Operator::Neq,
            "person",
            "name",
            None,
            &json!(""),
            FieldMetadataType::Text,
            false,
        )
        .unwrap();
        assert_eq!(
            parts.sql,
            "(\"person\".\"name\" != :name0000000000 AND \"person\".\"name\" IS NOT NULL)"
        );
        assert_eq!(parts.params.get("name0000000000"), Some(&json!("")));
        assert_eq!(
            run(
                Operator::Is,
                "person",
                "name",
                None,
                &json!("NOT_NULL"),
                FieldMetadataType::Text,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"name\" IS NOT NULL"
        );
        assert_eq!(
            run(
                Operator::Neq,
                "person",
                "companyId",
                None,
                &Value::Null,
                FieldMetadataType::Uuid,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"companyId\" IS NOT NULL"
        );
        assert_eq!(
            run(
                Operator::Like,
                "person",
                "name",
                None,
                &json!(""),
                FieldMetadataType::Text,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"name\"::text LIKE :name0000000000 OR \"person\".\"name\" IS NULL"
        );
        assert_eq!(
            run(
                Operator::Like,
                "person",
                "metadata",
                None,
                &json!(""),
                FieldMetadataType::RawJson,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"metadata\"::text LIKE :metadata0000000000"
        );
    }

    #[test]
    fn date_time_operators_use_the_half_open_interval() {
        assert_eq!(
            run(
                Operator::Eq,
                "person",
                "createdAt",
                None,
                &json!("2020-01-01T00:00:00.123Z"),
                FieldMetadataType::DateTime,
                false
            )
            .unwrap()
            .sql,
            "(\"person\".\"createdAt\" >= :createdAt0000000000 AND \"person\".\"createdAt\" < :createdAt0000000000::timestamptz + interval '1 millisecond')"
        );
        assert_eq!(
            run(
                Operator::Gt,
                "person",
                "createdAt",
                None,
                &json!("2020-01-01T00:00:00.123Z"),
                FieldMetadataType::DateTime,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"createdAt\" >= :createdAt0000000000::timestamptz + interval '1 millisecond'"
        );
        assert_eq!(
            run(
                Operator::Lte,
                "person",
                "createdAt",
                None,
                &json!("2020-01-01T00:00:00.123Z"),
                FieldMetadataType::DateTime,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"createdAt\" < :createdAt0000000000::timestamptz + interval '1 millisecond'"
        );
        assert_eq!(
            run(
                Operator::Eq,
                "person",
                "birthday",
                None,
                &json!("2020-01-01"),
                FieldMetadataType::Date,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"birthday\" = :birthday0000000000"
        );
    }

    #[test]
    fn strict_operators_never_widen() {
        assert_eq!(
            run(
                Operator::IsStrictly,
                "person",
                "name",
                None,
                &json!("NULL"),
                FieldMetadataType::Text,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"name\" IS NULL"
        );
        assert_eq!(
            run(
                Operator::IsStrictly,
                "person",
                "name",
                None,
                &json!("NOT_NULL"),
                FieldMetadataType::Text,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"name\" IS NOT NULL"
        );
        assert_eq!(
            run(
                Operator::EqStrict,
                "person",
                "name",
                None,
                &json!(""),
                FieldMetadataType::Text,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"name\" = :name0000000000"
        );
        assert_eq!(
            run(
                Operator::GtStrict,
                "person",
                "createdAt",
                None,
                &json!("2020-01-01T00:00:00.123Z"),
                FieldMetadataType::DateTime,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"createdAt\" > :createdAt0000000000"
        );
    }

    #[test]
    fn format_search_terms_builds_prefix_tsquery() {
        assert_eq!(
            format_search_terms("Ada Lovelace", "and"),
            "Ada:* & Lovelace:*"
        );
        assert_eq!(format_search_terms("a b", "or"), "a:* | b:*");
        assert_eq!(format_search_terms("  ", "and"), "");
        assert_eq!(
            format_search_terms("  hello   world ", "and"),
            "hello:* & world:*"
        );
        assert_eq!(
            format_search_terms("it's a & b", "and"),
            "it\\'s:* & a:* & \\&:* & b:*"
        );
        assert_eq!(format_search_terms("plain", "and"), "plain:*");
    }

    #[test]
    fn search_operator_binds_both_tsquery_and_like_parameters() {
        let parts = run(
            Operator::Search,
            "person",
            "searchVector",
            None,
            &json!("Ada Lovelace"),
            FieldMetadataType::TsVector,
            false,
        )
        .unwrap();
        let expected = "(\n          \"person\".\"searchVector\" @@ to_tsquery('simple', public.unaccent_immutable(:searchVector0000000000Ts)) OR\n          public.unaccent_immutable(\"person\".\"searchVector\"::text) ILIKE public.unaccent_immutable(:searchVector0000000000Like)\n        )";
        assert_eq!(parts.sql, expected);
        assert_eq!(
            parts.params.get("searchVector0000000000Ts"),
            Some(&json!("Ada:* & Lovelace:*"))
        );
        assert_eq!(
            parts.params.get("searchVector0000000000Like"),
            Some(&json!("%Ada Lovelace%"))
        );
    }

    #[test]
    fn spread_operators_emit_named_spread_parameters() {
        assert_eq!(
            run(
                Operator::In,
                "person",
                "name",
                None,
                &json!(["Ada", "Bob"]),
                FieldMetadataType::Text,
                false
            )
            .unwrap()
            .sql,
            "\"person\".\"name\" IN (:...name0000000000)"
        );
        assert_eq!(
            run(
                Operator::Contains,
                "pet",
                "tags",
                None,
                &json!(["a", "b"]),
                FieldMetadataType::Array,
                false
            )
            .unwrap()
            .sql,
            "\"pet\".\"tags\" @> ARRAY[:...tags0000000000]"
        );
        assert_eq!(
            run(
                Operator::NotContains,
                "pet",
                "tags",
                None,
                &json!(["a"]),
                FieldMetadataType::Array,
                false
            )
            .unwrap()
            .sql,
            "NOT (\"pet\".\"tags\"::text[] && ARRAY[:...tags0000000000]::text[])"
        );
        assert_eq!(
            run(
                Operator::ContainsAny,
                "pet",
                "traits",
                None,
                &json!(["x", "y"]),
                FieldMetadataType::MultiSelect,
                false
            )
            .unwrap()
            .sql,
            "\"pet\".\"traits\"::text[] && ARRAY[:...traits0000000000]::text[]"
        );
    }

    #[test]
    fn is_empty_array_operators_match_the_array_null_equivalent() {
        assert_eq!(
            run(
                Operator::IsEmptyArray,
                "pet",
                "traits",
                None,
                &json!(true),
                FieldMetadataType::Array,
                false
            )
            .unwrap()
            .sql,
            "(\"pet\".\"traits\" = '{}' OR \"pet\".\"traits\" IS NULL)"
        );
        assert_eq!(
            run(
                Operator::IsEmptyArray,
                "pet",
                "traits",
                None,
                &json!(false),
                FieldMetadataType::Array,
                false
            )
            .unwrap()
            .sql,
            "(\"pet\".\"traits\" IS NOT NULL AND \"pet\".\"traits\" != '{}')"
        );
    }

    #[test]
    fn like_style_operators_coerce_values_with_js_string_semantics() {
        let parts = run(
            Operator::Like,
            "person",
            "age",
            None,
            &json!(42),
            FieldMetadataType::Number,
            false,
        )
        .unwrap();
        assert_eq!(parts.params.get("age0000000000"), Some(&json!("42")));
        let parts = run(
            Operator::StartsWith,
            "person",
            "age",
            None,
            &json!(true),
            FieldMetadataType::Number,
            false,
        )
        .unwrap();
        assert_eq!(parts.params.get("age0000000000"), Some(&json!("true")));
    }

    #[test]
    fn direct_table_reference_drops_the_object_prefix() {
        let parts = run(
            Operator::Eq,
            "person",
            "name",
            None,
            &json!("Ada"),
            FieldMetadataType::Text,
            true,
        )
        .unwrap();
        assert_eq!(parts.sql, "\"name\" = :name0000000000");
    }

    #[test]
    fn js_string_matches_the_number_to_string_output() {
        assert_eq!(js_string(&json!(1.5)), Value::String("1.5".to_string()));
        assert_eq!(js_string(&json!(-3)), Value::String("-3".to_string()));
        assert_eq!(js_string(&json!(true)), Value::String("true".to_string()));
        assert_eq!(js_string(&json!(null)), Value::String("null".to_string()));
        assert_eq!(
            js_string(&json!(["a", "b"])),
            Value::String("a,b".to_string())
        );
        assert_eq!(
            js_string(&json!({"x": 1})),
            Value::String("[object Object]".to_string())
        );
    }
}
