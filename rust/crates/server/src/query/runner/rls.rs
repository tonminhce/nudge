use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use sqlx::Executor;
use sqlx::PgPool;
use uuid::Uuid;

use crate::metadata::flat::FieldMetadataType;
use crate::metadata::names::compute_composite_column_name;
use crate::permissions::ObjectRecordsOperationType;
use crate::permissions::RowLevelRecordsPermissions;
use crate::query::filter::CompiledFilter;
use crate::query::repository::quote_identifier;

use super::base::compile_runner_filter;
use super::base::RunnerSchema;
use super::RunnerError;

pub async fn row_access_filter(
    pool: &PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    operation: ObjectRecordsOperationType,
    suffixes: &mut crate::query::filter::Suffixes,
) -> Result<Option<CompiledFilter>, RunnerError> {
    let model = schema.object(object_name_singular)?;
    let Some(permissions) = schema.records_permissions() else {
        return Ok(None);
    };
    let Some(object) = permissions.object(model.object.id) else {
        return Ok(None);
    };
    let operation = operation.enforced();
    if object.operation_allowed(operation) {
        return Ok(None);
    }
    let rules = object
        .row_level_records_permissions
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|rule| rule_allows(rule, operation))
        .collect::<Vec<_>>();
    if rules.is_empty() {
        return Ok(Some(unsatisfiable_filter()));
    }
    let mut member_values = HashMap::new();
    let mut filters = Vec::with_capacity(rules.len());
    for rule in rules {
        filters.push(
            compile_rule_value(pool, schema, object_name_singular, rule, &mut member_values)
                .await?,
        );
    }
    let filter = if filters.len() == 1 {
        filters.remove(0)
    } else {
        json!({ "or": filters })
    };
    compile_runner_filter(schema, object_name_singular, Some(&filter), suffixes)
}

pub fn requires_row_access(
    schema: &RunnerSchema,
    object_name_singular: &str,
    operation: ObjectRecordsOperationType,
) -> Result<bool, RunnerError> {
    let model = schema.object(object_name_singular)?;
    let Some(permissions) = schema.records_permissions() else {
        return Ok(false);
    };
    let Some(object) = permissions.object(model.object.id) else {
        return Ok(false);
    };
    let operation = operation.enforced();
    Ok(!object.operation_allowed(operation)
        && object
            .row_level_records_permissions
            .as_ref()
            .into_iter()
            .flatten()
            .any(|rule| rule_allows(rule, operation)))
}

pub async fn validate_created_rows<'e, E>(
    executor: E,
    pool: &PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    ids: &[Uuid],
) -> Result<(), RunnerError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let Some(access) = row_access_filter(
        pool,
        schema,
        object_name_singular,
        ObjectRecordsOperationType::Create,
        &mut crate::query::filter::Suffixes::new(),
    )
    .await?
    else {
        return Ok(());
    };
    let model = schema.object(object_name_singular)?;
    let condition = super::base::resolve_where_clauses(
        schema,
        object_name_singular,
        object_name_singular,
        &access.clauses,
    )?;
    let mut parameters = Map::new();
    parameters.insert(
        String::from("ids"),
        Value::Array(ids.iter().map(|id| Value::String(id.to_string())).collect()),
    );
    parameters.extend(access.params);
    let sql = format!(
        "SELECT {} FROM {}.{} WHERE {} = ANY(ARRAY[:...ids]::uuid[]) AND ({condition})",
        quote_identifier("id")?,
        quote_identifier(&model.shape.schema_name)?,
        quote_identifier(&model.shape.table_name)?,
        quote_identifier("id")?,
    );
    let compiled = crate::query::compile::compile_named_parameters(&sql, &parameters)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let mut query = sqlx::query_scalar::<_, Uuid>(&compiled.text);
    for value in &compiled.values {
        query = bind_access_value(query, value)?;
    }
    let visible = query
        .fetch_all(executor)
        .await
        .map_err(RunnerError::database)?;
    if visible.len() != ids.len() {
        return Err(RunnerError::new(
            "RECORD_DOES_NOT_SATISFY_RLS",
            "Record does not satisfy row-level security constraints of your current role",
        ));
    }
    Ok(())
}

fn bind_access_value<'q>(
    query: sqlx::query::QueryScalar<'q, sqlx::Postgres, Uuid, sqlx::postgres::PgArguments>,
    value: &'q Value,
) -> Result<
    sqlx::query::QueryScalar<'q, sqlx::Postgres, Uuid, sqlx::postgres::PgArguments>,
    RunnerError,
> {
    Ok(match value {
        Value::Null => query.bind(Option::<String>::None),
        Value::Bool(value) => query.bind(*value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                query.bind(value)
            } else if let Some(value) = value.as_u64() {
                query.bind(i64::try_from(value).map_err(|_| RunnerError::invalid_parameter())?)
            } else {
                query.bind(value.as_f64().unwrap_or_default())
            }
        }
        Value::String(value) => query.bind(value.as_str()),
        Value::Array(items) => query.bind(
            items
                .iter()
                .map(|item| item.as_str().unwrap_or_default().to_string())
                .collect::<Vec<_>>(),
        ),
        Value::Object(_) => query.bind(value.to_string()),
    })
}

fn rule_allows(rule: &RowLevelRecordsPermissions, operation: ObjectRecordsOperationType) -> bool {
    match operation {
        ObjectRecordsOperationType::Create => rule.can_create_records,
        ObjectRecordsOperationType::Read => rule.can_read_records,
        ObjectRecordsOperationType::Update => rule.can_update_records,
        ObjectRecordsOperationType::SoftDelete => rule.can_soft_delete_records,
        ObjectRecordsOperationType::Destroy => rule.can_destroy_records,
    }
}

fn unsatisfiable_filter() -> CompiledFilter {
    CompiledFilter {
        clauses: vec![crate::query::filter::WhereClause::sql(
            crate::query::filter::JoinOp::And,
            "FALSE",
        )],
        params: Vec::new(),
    }
}

async fn compile_rule_value(
    pool: &PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    rule: &RowLevelRecordsPermissions,
    member_values: &mut HashMap<String, Value>,
) -> Result<Value, RunnerError> {
    let predicates = rule.predicates.as_slice();
    let groups = rule.predicate_groups.as_slice();
    let mut filters = Vec::new();
    for predicate in predicates {
        if predicate_group_id(predicate).is_none() {
            filters.push(
                compile_predicate_value(
                    pool,
                    schema,
                    object_name_singular,
                    predicate,
                    member_values,
                )
                .await?,
            );
        }
    }
    let mut visiting = HashSet::new();
    for group in groups {
        if parent_group_id(group).is_none() {
            filters.push(
                compile_group_value(
                    pool,
                    schema,
                    object_name_singular,
                    predicates,
                    groups,
                    group,
                    member_values,
                    &mut visiting,
                )
                .await?,
            );
        }
    }
    Ok(combine_filters("AND", filters))
}

#[allow(clippy::too_many_arguments)]
async fn compile_group_value(
    pool: &PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    predicates: &[Value],
    groups: &[Value],
    group: &Value,
    member_values: &mut HashMap<String, Value>,
    visiting: &mut HashSet<String>,
) -> Result<Value, RunnerError> {
    let group_id = required_string(group, "id")?;
    if !visiting.insert(group_id.to_string()) {
        return Err(invalid(
            "Row-level permission predicate group cycle detected",
        ));
    }
    let mut filters = Vec::new();
    for predicate in predicates {
        if predicate_group_id(predicate).as_deref() == Some(group_id) {
            filters.push(
                compile_predicate_value(
                    pool,
                    schema,
                    object_name_singular,
                    predicate,
                    member_values,
                )
                .await?,
            );
        }
    }
    for child in groups {
        if parent_group_id(child).as_deref() == Some(group_id) {
            filters.push(
                Box::pin(compile_group_value(
                    pool,
                    schema,
                    object_name_singular,
                    predicates,
                    groups,
                    child,
                    member_values,
                    visiting,
                ))
                .await?,
            );
        }
    }
    visiting.remove(group_id);
    let operator = group
        .get("logicalOperator")
        .and_then(Value::as_str)
        .unwrap_or("AND");
    Ok(combine_filters(operator, filters))
}

fn combine_filters(operator: &str, filters: Vec<Value>) -> Value {
    match filters.len() {
        0 => unsatisfiable_value(),
        1 => filters.into_iter().next().expect("one filter"),
        _ if operator == "OR" => json!({ "or": filters }),
        _ => json!({ "and": filters }),
    }
}

async fn compile_predicate_value(
    pool: &PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    predicate: &Value,
    member_values: &mut HashMap<String, Value>,
) -> Result<Value, RunnerError> {
    let field_id = required_uuid(predicate, "fieldMetadataId")?;
    let field = schema
        .field_metadata(field_id)
        .ok_or_else(|| invalid("Field metadata not found for row-level predicate"))?;
    let operand = required_string(predicate, "operand")?;
    let sub_field_name = optional_string(predicate, "subFieldName");
    let mut value = predicate.get("value").cloned().unwrap_or(Value::Null);
    if let Some(member_field_id) = optional_uuid(predicate, "workspaceMemberFieldMetadataId")? {
        value =
            resolve_member_value(pool, schema, predicate, member_field_id, member_values).await?;
        if value.is_null() {
            return Ok(unsatisfiable_value());
        }
    }
    if !matches!(operand, "IS_NOT_NULL" | "IS_EMPTY" | "IS_NOT_EMPTY")
        && (value.is_null()
            || value.as_str() == Some("")
            || value.as_str() == Some("[]")
            || value.as_array().is_some_and(Vec::is_empty))
    {
        return Ok(unsatisfiable_value());
    }
    let full_name = field.field_type == FieldMetadataType::FullName && sub_field_name.is_none();
    let raw_filter = (!full_name)
        .then(|| {
            operand_filter(
                field.field_type,
                operand,
                sub_field_name.as_deref(),
                value.clone(),
            )
        })
        .transpose()?;
    if let Some(relation_field_id) = optional_uuid(predicate, "relationFieldMetadataId")? {
        let relation_field = schema
            .field_metadata(relation_field_id)
            .ok_or_else(|| invalid("Relation field metadata not found for row-level predicate"))?;
        let filter = if full_name {
            full_name_filter(&field.name, operand, value)?
        } else {
            with_sub_field(
                &field.name,
                sub_field_name.as_deref(),
                raw_filter.expect("non-full-name filter"),
            )
        };
        return Ok(json!({ (relation_field.name.as_str()): filter }));
    }
    if matches!(
        field.field_type,
        FieldMetadataType::Relation | FieldMetadataType::MorphRelation
    ) {
        let model = schema.object(object_name_singular)?;
        let join_column = model
            .shape
            .relation(&field.name)
            .and_then(|relation| relation.join_column_name.clone())
            .ok_or_else(|| invalid("Relation join column not found for row-level predicate"))?;
        return Ok(json!({ (join_column): raw_filter.expect("relation filter") }));
    }
    if full_name {
        return full_name_filter(&field.name, operand, value);
    }
    Ok(with_sub_field(
        &field.name,
        sub_field_name.as_deref(),
        raw_filter.expect("non-full-name filter"),
    ))
}

fn operand_filter(
    field_type: FieldMetadataType,
    operand: &str,
    sub_field_name: Option<&str>,
    value: Value,
) -> Result<Value, RunnerError> {
    if matches!(operand, "IS_EMPTY" | "IS_NOT_EMPTY") {
        let empty = if matches!(
            field_type,
            FieldMetadataType::Array | FieldMetadataType::MultiSelect
        ) {
            json!({ "isEmptyArray": true })
        } else {
            json!({ "is": "NULL" })
        };
        return Ok(if operand == "IS_NOT_EMPTY" {
            json!({ "not": empty })
        } else {
            empty
        });
    }
    if operand == "IS_NOT_NULL" {
        return Ok(json!({ "is": "NOT_NULL" }));
    }
    let operator = match operand {
        "IS" => "eq",
        "IS_NOT" => "neq",
        "LESS_THAN_OR_EQUAL" => "lte",
        "GREATER_THAN_OR_EQUAL" => "gte",
        "IS_BEFORE" => "lt",
        "IS_AFTER" => "gte",
        "CONTAINS" => {
            return match field_type {
                FieldMetadataType::MultiSelect => Ok(json!({ "containsAny": as_array(value) })),
                FieldMetadataType::Array => {
                    Ok(json!({ "containsIlike": contains_pattern(value)? }))
                }
                FieldMetadataType::Text | FieldMetadataType::RawJson | FieldMetadataType::Files
                    if sub_field_name.is_none() =>
                {
                    Ok(json!({ "ilike": contains_pattern(value)? }))
                }
                FieldMetadataType::FullName
                | FieldMetadataType::Emails
                | FieldMetadataType::Phones
                | FieldMetadataType::Links
                | FieldMetadataType::Address
                    if sub_field_name.is_some() =>
                {
                    Ok(json!({ "ilike": contains_pattern(value)? }))
                }
                _ => Err(invalid(format!(
                    "Operand {operand} is not supported for {} row-level predicates",
                    field_type.as_str()
                ))),
            };
        }
        "DOES_NOT_CONTAIN" => {
            return match field_type {
                FieldMetadataType::MultiSelect => Ok(json!({
                    "or": [
                        { "not": { "containsAny": as_array(value) } },
                        { "isEmptyArray": true },
                        { "is": "NULL" }
                    ]
                })),
                FieldMetadataType::Array => {
                    Ok(json!({ "not": { "containsIlike": contains_pattern(value)? } }))
                }
                FieldMetadataType::Text => {
                    Ok(json!({ "not": { "ilike": contains_pattern(value)? } }))
                }
                FieldMetadataType::FullName
                | FieldMetadataType::Emails
                | FieldMetadataType::Phones
                | FieldMetadataType::Links
                | FieldMetadataType::Address
                    if sub_field_name.is_some() =>
                {
                    Ok(json!({ "not": { "ilike": contains_pattern(value)? } }))
                }
                FieldMetadataType::RawJson | FieldMetadataType::Files => Ok(json!({
                    "or": [
                        { "not": { "ilike": contains_pattern(value)? } },
                        { "is": "NULL" }
                    ]
                })),
                _ => Err(invalid(format!(
                    "Operand {operand} is not supported for {} row-level predicates",
                    field_type.as_str()
                ))),
            };
        }
        _ => {
            return Err(invalid(format!(
                "Unsupported row-level permission operand {operand}"
            )))
        }
    };
    let set_field = matches!(
        field_type,
        FieldMetadataType::Select
            | FieldMetadataType::MultiSelect
            | FieldMetadataType::Uuid
            | FieldMetadataType::Relation
    );
    if set_field && operand == "IS_NOT" {
        return Ok(json!({ "not": { "in": as_array(value) } }));
    }
    let value = if set_field { as_array(value) } else { value };
    let operator = if set_field && operand == "IS" {
        "in"
    } else {
        operator
    };
    let _ = sub_field_name;
    Ok(json!({ (operator): value }))
}

fn full_name_filter(field_name: &str, operand: &str, value: Value) -> Result<Value, RunnerError> {
    if operand != "CONTAINS" {
        return Err(invalid(
            "Full-name row-level predicates require a sub field",
        ));
    }
    let pattern = contains_pattern(value)?;
    Ok(json!({
        "or": [
            { (field_name): { "firstName": { "ilike": pattern } } },
            { (field_name): { "lastName": { "ilike": pattern } } }
        ]
    }))
}

fn with_sub_field(field_name: &str, sub_field_name: Option<&str>, filter: Value) -> Value {
    match sub_field_name {
        Some(sub_field_name) => json!({ (field_name): { (sub_field_name): filter } }),
        None => json!({ (field_name): filter }),
    }
}

fn as_array(value: Value) -> Value {
    if value.is_array() {
        value
    } else {
        Value::Array(vec![value])
    }
}

fn contains_pattern(value: Value) -> Result<Value, RunnerError> {
    value
        .as_str()
        .map(|value| Value::String(format!("%{value}%")))
        .ok_or_else(|| invalid("Row-level permission contains operand requires a string"))
}

fn unsatisfiable_value() -> Value {
    json!({ "id": { "is": "NULL" } })
}

async fn resolve_member_value(
    pool: &PgPool,
    schema: &RunnerSchema,
    predicate: &Value,
    member_field_id: Uuid,
    member_values: &mut HashMap<String, Value>,
) -> Result<Value, RunnerError> {
    let member_sub_field = optional_string(predicate, "workspaceMemberSubFieldName");
    let member_relation_field_id =
        optional_uuid(predicate, "workspaceMemberRelationFieldMetadataId")?;
    let key = format!(
        "{}{}:{}",
        member_relation_field_id
            .map(|id| format!("{id}>"))
            .unwrap_or_default(),
        member_field_id,
        member_sub_field.as_deref().unwrap_or_default()
    );
    if let Some(value) = member_values.get(&key) {
        return Ok(value.clone());
    }
    let Some(member_id) = schema.workspace_member_id() else {
        member_values.insert(key, Value::Null);
        return Ok(Value::Null);
    };
    let field = schema.field_metadata(member_field_id).ok_or_else(|| {
        invalid("Workspace member field metadata not found for row-level predicate")
    })?;
    let value = if let Some(relation_field_id) = member_relation_field_id {
        resolve_member_relation_value(
            pool,
            schema,
            member_id,
            relation_field_id,
            field,
            member_sub_field.as_deref(),
        )
        .await?
    } else {
        let member_model = schema.object_by_id(&field.object_metadata_id)?;
        let column_name = member_column_name(field, member_sub_field.as_deref());
        let member_table = format!(
            "{}.{}",
            quote_identifier(&member_model.shape.schema_name)?,
            quote_identifier(&member_model.shape.table_name)?
        );
        sqlx::query_scalar::<_, Option<Value>>(&format!(
            "SELECT to_jsonb(member)->$2 FROM {member_table} member WHERE member.id = $1"
        ))
        .bind(member_id)
        .bind(column_name)
        .fetch_optional(pool)
        .await
        .map_err(RunnerError::database)?
        .flatten()
        .unwrap_or(Value::Null)
    };
    member_values.insert(key, value.clone());
    Ok(value)
}

async fn resolve_member_relation_value(
    pool: &PgPool,
    schema: &RunnerSchema,
    member_id: Uuid,
    relation_field_id: Uuid,
    target_field: &crate::metadata::flat::FlatFieldMetadata,
    sub_field_name: Option<&str>,
) -> Result<Value, RunnerError> {
    let relation_field = schema
        .field_metadata(relation_field_id)
        .ok_or_else(|| invalid("Workspace member relation field metadata not found"))?;
    let member_model = schema.object_by_id(&relation_field.object_metadata_id)?;
    let relation = member_model
        .shape
        .relation(&relation_field.name)
        .ok_or_else(|| invalid("Workspace member relation shape not found"))?;
    let target_model = schema.object_by_id(&target_field.object_metadata_id)?;
    let target_column = quote_identifier(&member_column_name(target_field, sub_field_name))?;
    let target_table = format!(
        "{}.{}",
        quote_identifier(&target_model.shape.schema_name)?,
        quote_identifier(&target_model.shape.table_name)?
    );
    let member_table = format!(
        "{}.{}",
        quote_identifier(&member_model.shape.schema_name)?,
        quote_identifier(&member_model.shape.table_name)?
    );
    let value = if let Some(join_column) = relation.join_column_name.as_deref() {
        let join_column = quote_identifier(join_column)?;
        let sql = format!(
            "SELECT COALESCE(jsonb_agg(target.{target_column}) FILTER (WHERE target.{target_column} IS NOT NULL), '[]'::jsonb) \
             FROM {target_table} target \
             WHERE target.id = (SELECT member.{join_column} FROM {member_table} member WHERE member.id = $1)"
        );
        sqlx::query_scalar::<_, Value>(&sql)
            .bind(member_id)
            .fetch_one(pool)
            .await
            .map_err(RunnerError::database)?
    } else {
        let child_join_column = quote_identifier(
            &schema.child_join_column(&member_model.object.name_singular, &relation_field.name)?,
        )?;
        let sql = format!(
            "SELECT COALESCE(jsonb_agg(target.{target_column}) FILTER (WHERE target.{target_column} IS NOT NULL), '[]'::jsonb) \
             FROM {target_table} target WHERE target.{child_join_column} = $1"
        );
        sqlx::query_scalar::<_, Value>(&sql)
            .bind(member_id)
            .fetch_one(pool)
            .await
            .map_err(RunnerError::database)?
    };
    Ok(value)
}

fn member_column_name(
    field: &crate::metadata::flat::FlatFieldMetadata,
    sub_field_name: Option<&str>,
) -> String {
    sub_field_name
        .map(|sub_field_name| compute_composite_column_name(&field.name, sub_field_name))
        .unwrap_or_else(|| field.name.clone())
}

fn predicate_group_id(predicate: &Value) -> Option<String> {
    optional_string(predicate, "rowLevelPermissionPredicateGroupId")
}

fn parent_group_id(group: &Value) -> Option<String> {
    optional_string(group, "parentRowLevelPermissionPredicateGroupId")
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, RunnerError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("Missing row-level permission field {key}")))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn required_uuid(value: &Value, key: &str) -> Result<Uuid, RunnerError> {
    Uuid::parse_str(required_string(value, key)?)
        .map_err(|_| invalid(format!("Invalid row-level permission field {key}")))
}

fn optional_uuid(value: &Value, key: &str) -> Result<Option<Uuid>, RunnerError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|value| {
            Uuid::parse_str(value)
                .map_err(|_| invalid(format!("Invalid row-level permission field {key}")))
        })
        .transpose()
}

fn invalid(message: impl Into<String>) -> RunnerError {
    RunnerError::new("RECORD_PERMISSION_QUERY_INVALID", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::flat::FieldMetadataType;

    #[test]
    fn root_and_nested_filters_use_the_group_operator() {
        assert_eq!(
            combine_filters(
                "OR",
                vec![
                    json!({ "status": { "eq": "OPEN" } }),
                    json!({ "priority": { "gte": 3 } })
                ],
            ),
            json!({
                "or": [
                    { "status": { "eq": "OPEN" } },
                    { "priority": { "gte": 3 } }
                ]
            })
        );
        assert_eq!(combine_filters("AND", Vec::new()), unsatisfiable_value());
    }

    #[test]
    fn scalar_set_and_contains_operands_match_record_filters() {
        assert_eq!(
            operand_filter(
                FieldMetadataType::Number,
                "GREATER_THAN_OR_EQUAL",
                None,
                json!(3)
            )
            .unwrap(),
            json!({ "gte": 3 })
        );
        assert_eq!(
            operand_filter(FieldMetadataType::Select, "IS", None, json!("OPEN")).unwrap(),
            json!({ "in": ["OPEN"] })
        );
        assert_eq!(
            operand_filter(FieldMetadataType::Text, "CONTAINS", None, json!("Ivan")).unwrap(),
            json!({ "ilike": "%Ivan%" })
        );
        assert_eq!(
            operand_filter(
                FieldMetadataType::MultiSelect,
                "IS_NOT_EMPTY",
                None,
                Value::Null
            )
            .unwrap(),
            json!({ "not": { "isEmptyArray": true } })
        );
    }

    #[test]
    fn predicate_fields_wrap_exactly_once() {
        assert_eq!(
            with_sub_field(
                "id",
                None,
                json!({ "in": ["7a93d1e5-3f74-4945-8a65-d7f996083f72"] })
            ),
            json!({ "id": { "in": ["7a93d1e5-3f74-4945-8a65-d7f996083f72"] } })
        );
        assert_eq!(
            with_sub_field("name", Some("firstName"), json!({ "eq": "Ivan" })),
            json!({ "name": { "firstName": { "eq": "Ivan" } } })
        );
    }

    #[test]
    fn unsupported_relative_operand_fails_closed() {
        assert_eq!(
            operand_filter(FieldMetadataType::Date, "IS_RELATIVE", None, json!("TODAY"))
                .unwrap_err()
                .code(),
            "RECORD_PERMISSION_QUERY_INVALID"
        );
    }
}
