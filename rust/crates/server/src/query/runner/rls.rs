use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use sqlx::Executor;
use sqlx::PgPool;
use uuid::Uuid;

use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::names::compute_composite_column_name;
use crate::permissions::ObjectRecordsOperationType;
use crate::permissions::RowLevelRecordsPermissions;
use crate::query::filter::compile_filter;
use crate::query::filter::CompiledFilter;
use crate::query::filter::FilterKeyInfo;
use crate::query::filter::FilterSchema;
use crate::query::filter::JoinOp;
use crate::query::filter::Suffixes;
use crate::query::filter::WhereClause;
use crate::query::repository::quote_identifier;

use super::base::ObjectModel;
use super::base::RunnerSchema;
use super::RunnerError;

mod relative_date;

pub async fn row_access_filter(
    pool: &PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    operation: ObjectRecordsOperationType,
    suffixes: &mut Suffixes,
) -> Result<Option<CompiledFilter>, RunnerError> {
    row_access_filter_with_alias(
        pool,
        schema,
        object_name_singular,
        operation,
        object_name_singular,
        suffixes,
    )
    .await
}

pub(super) async fn row_access_filter_with_alias(
    pool: &PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    operation: ObjectRecordsOperationType,
    alias: &str,
    suffixes: &mut Suffixes,
) -> Result<Option<CompiledFilter>, RunnerError> {
    let mut compiler = AccessCompiler {
        pool,
        schema,
        suffixes,
        member_values: HashMap::new(),
        relation_stack: HashSet::new(),
    };
    compiler
        .compile_access(object_name_singular, operation, alias, false)
        .await
}

struct AccessCompiler<'a> {
    pool: &'a PgPool,
    schema: &'a RunnerSchema,
    suffixes: &'a mut Suffixes,
    member_values: HashMap<String, Value>,
    relation_stack: HashSet<Uuid>,
}

struct AliasedFilterSchema<'a> {
    schema: &'a RunnerSchema,
    object_name: &'a str,
}

struct RelationPredicate<'a> {
    field: &'a crate::metadata::flat::FlatFieldMetadata,
    relation_field_id: Uuid,
    full_name: bool,
    operand: &'a str,
    value: Value,
    raw_filter: Option<Value>,
    top_level_filter: Option<Value>,
    sub_field_name: Option<&'a str>,
}

impl FilterSchema for AliasedFilterSchema<'_> {
    fn lookup(&self, _object_name: &str, key: &str) -> Option<FilterKeyInfo> {
        self.schema.lookup(self.object_name, key)
    }
}

impl AccessCompiler<'_> {
    async fn compile_access(
        &mut self,
        object_name: &str,
        operation: ObjectRecordsOperationType,
        alias: &str,
        deny_missing: bool,
    ) -> Result<Option<CompiledFilter>, RunnerError> {
        let model = self.schema.object(object_name)?;
        let Some(permissions) = self.schema.records_permissions() else {
            return Ok(None);
        };
        let Some(object) = permissions.object(model.object.id) else {
            return Ok(deny_missing.then(unsatisfiable_filter));
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
        if !self.relation_stack.insert(model.object.id) {
            return Err(invalid("Row-level permission relation cycle detected"));
        }
        let result = self.compile_rules(object_name, alias, &rules).await;
        self.relation_stack.remove(&model.object.id);
        Ok(Some(result?))
    }

    async fn compile_rules(
        &mut self,
        object_name: &str,
        alias: &str,
        rules: &[&RowLevelRecordsPermissions],
    ) -> Result<CompiledFilter, RunnerError> {
        let mut filters = Vec::with_capacity(rules.len());
        for rule in rules {
            filters.push(self.compile_rule(object_name, alias, rule).await?);
        }
        Ok(combine_compiled_filters(JoinOp::Or, filters))
    }

    async fn compile_rule(
        &mut self,
        object_name: &str,
        alias: &str,
        rule: &RowLevelRecordsPermissions,
    ) -> Result<CompiledFilter, RunnerError> {
        let predicates = rule.predicates.as_slice();
        let groups = rule.predicate_groups.as_slice();
        let mut filters = Vec::new();
        for predicate in predicates {
            if predicate_group_id(predicate).is_none() {
                filters.push(
                    self.compile_predicate(object_name, alias, predicate)
                        .await?,
                );
            }
        }
        let mut visiting = HashSet::new();
        for group in groups {
            if parent_group_id(group).is_none() {
                filters.push(
                    self.compile_group(
                        object_name,
                        alias,
                        predicates,
                        groups,
                        group,
                        &mut visiting,
                    )
                    .await?,
                );
            }
        }
        Ok(combine_compiled_filters(JoinOp::And, filters))
    }

    async fn compile_group(
        &mut self,
        object_name: &str,
        alias: &str,
        predicates: &[Value],
        groups: &[Value],
        group: &Value,
        visiting: &mut HashSet<String>,
    ) -> Result<CompiledFilter, RunnerError> {
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
                    self.compile_predicate(object_name, alias, predicate)
                        .await?,
                );
            }
        }
        for child in groups {
            if parent_group_id(child).as_deref() == Some(group_id) {
                filters.push(
                    Box::pin(self.compile_group(
                        object_name,
                        alias,
                        predicates,
                        groups,
                        child,
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
        let operator = if operator == "OR" {
            JoinOp::Or
        } else {
            JoinOp::And
        };
        Ok(combine_compiled_filters(operator, filters))
    }

    async fn compile_predicate(
        &mut self,
        object_name: &str,
        alias: &str,
        predicate: &Value,
    ) -> Result<CompiledFilter, RunnerError> {
        let field_id = required_uuid(predicate, "fieldMetadataId")?;
        let field = self
            .schema
            .field_metadata(field_id)
            .ok_or_else(|| invalid("Field metadata not found for row-level predicate"))?;
        let operand = required_string(predicate, "operand")?;
        let sub_field_name = optional_string(predicate, "subFieldName");
        let relation_field_id = optional_uuid(predicate, "relationFieldMetadataId")?;
        let member_relation_field_id =
            optional_uuid(predicate, "workspaceMemberRelationFieldMetadataId")?;
        if (relation_field_id.is_some() || member_relation_field_id.is_some())
            && !self
                .schema
                .records_permissions()
                .is_some_and(|permissions| permissions.relation_hop_enabled)
        {
            return Ok(unsatisfiable_filter());
        }
        let mut value = predicate.get("value").cloned().unwrap_or(Value::Null);
        if let Some(member_field_id) = optional_uuid(predicate, "workspaceMemberFieldMetadataId")? {
            value = resolve_member_value(
                self.pool,
                self.schema,
                predicate,
                member_field_id,
                &mut self.member_values,
            )
            .await?;
            if value.is_null() {
                return Ok(unsatisfiable_filter());
            }
        }
        let value = if field.field_type == FieldMetadataType::MorphRelation {
            morph_filter_value(value)?
        } else {
            value
        };
        if !matches!(
            operand,
            "IS_NOT_NULL"
                | "IS_EMPTY"
                | "IS_NOT_EMPTY"
                | "IS_IN_PAST"
                | "IS_IN_FUTURE"
                | "IS_TODAY"
        ) && (value.is_null()
            || value.as_str() == Some("")
            || value.as_str() == Some("[]")
            || value.as_array().is_some_and(Vec::is_empty))
        {
            return Ok(unsatisfiable_filter());
        }
        let full_name = field.field_type == FieldMetadataType::FullName && sub_field_name.is_none();
        let top_level_filter = (operand == "IS_RELATIVE")
            .then(|| {
                relative_date::resolve_filter(
                    &value,
                    field.field_type == FieldMetadataType::DateTime,
                    chrono::Utc::now(),
                )
                .map(|filter| with_relative_field(filter, &field.name))
            })
            .transpose()
            .map_err(invalid)?;
        let raw_filter = (top_level_filter.is_none() && !full_name)
            .then(|| {
                operand_filter(
                    field.field_type,
                    operand,
                    sub_field_name.as_deref(),
                    value.clone(),
                )
            })
            .transpose()?;
        if let Some(relation_field_id) = relation_field_id {
            return self
                .compile_relation_predicate(
                    object_name,
                    alias,
                    RelationPredicate {
                        field,
                        relation_field_id,
                        full_name,
                        operand,
                        value,
                        raw_filter,
                        top_level_filter,
                        sub_field_name: sub_field_name.as_deref(),
                    },
                )
                .await;
        }
        if let Some(filter) = top_level_filter {
            return self.compile_value_filter(object_name, alias, filter);
        }
        let filter = if field.field_type == FieldMetadataType::Relation {
            let model = self.schema.object(object_name)?;
            let join_column = model
                .shape
                .relation(&field.name)
                .and_then(|relation| relation.join_column_name.clone())
                .ok_or_else(|| invalid("Relation join column not found for row-level predicate"))?;
            json!({ (join_column): raw_filter.expect("relation filter") })
        } else if field.field_type == FieldMetadataType::MorphRelation {
            let model = self.schema.object(object_name)?;
            morph_relation_filter(
                model,
                field,
                raw_filter.expect("morph relation filter"),
                !matches!(
                    operand,
                    "IS_EMPTY" | "IS_NOT_EMPTY" | "IS_NULL" | "IS_NOT_NULL"
                ),
            )?
        } else if full_name {
            full_name_filter(&field.name, operand, value)?
        } else {
            with_sub_field(
                &field.name,
                sub_field_name.as_deref(),
                raw_filter.expect("non-full-name filter"),
            )
        };
        self.compile_value_filter(object_name, alias, filter)
    }

    async fn compile_relation_predicate(
        &mut self,
        object_name: &str,
        alias: &str,
        predicate: RelationPredicate<'_>,
    ) -> Result<CompiledFilter, RunnerError> {
        let RelationPredicate {
            field,
            relation_field_id,
            full_name,
            operand,
            value,
            raw_filter,
            top_level_filter,
            sub_field_name,
        } = predicate;
        let model = self.schema.object(object_name)?;
        let relation_field = self
            .schema
            .field_metadata(relation_field_id)
            .ok_or_else(|| invalid("Relation field metadata not found for row-level predicate"))?;
        if relation_field.object_metadata_id != model.object.id {
            return Err(invalid(
                "Relation field metadata does not belong to the row-level predicate object",
            ));
        }
        let (_, relation, target) = self.schema.relation(object_name, &relation_field.name)?;
        if field.object_metadata_id != target.object.id {
            return Err(invalid(
                "Row-level relation predicate field does not belong to the target object",
            ));
        }
        let filter = if let Some(filter) = top_level_filter {
            filter
        } else if full_name {
            full_name_filter(&field.name, operand, value)?
        } else {
            with_sub_field(
                &field.name,
                sub_field_name,
                raw_filter.expect("non-full-name filter"),
            )
        };
        let target_alias = format!("rls_{}", self.suffixes.take());
        let predicate =
            self.compile_value_filter(&target.object.name_singular, &target_alias, filter)?;
        let target_access = Box::pin(self.compile_access(
            &target.object.name_singular,
            ObjectRecordsOperationType::Read,
            &target_alias,
            true,
        ))
        .await?;
        let predicate_sql = super::base::resolve_where_clauses(
            self.schema,
            &target.object.name_singular,
            &target_alias,
            &predicate.clauses,
        )?;
        let mut conditions = vec![
            if let Some(join_column) = relation.join_column_name.as_deref() {
                format!(
                    "{}.{} = {}.{}",
                    quote_identifier(&target_alias)?,
                    quote_identifier("id")?,
                    quote_identifier(alias)?,
                    quote_identifier(join_column)?
                )
            } else {
                let child_join_column = self
                    .schema
                    .child_join_column(object_name, &relation_field.name)?;
                format!(
                    "{}.{} = {}.{}",
                    quote_identifier(&target_alias)?,
                    quote_identifier(&child_join_column)?,
                    quote_identifier(alias)?,
                    quote_identifier("id")?
                )
            },
        ];
        if target.shape.has_deleted_at_column {
            conditions.push(format!(
                "{}.{} IS NULL",
                quote_identifier(&target_alias)?,
                quote_identifier("deletedAt")?
            ));
        }
        conditions.push(predicate_sql);
        let mut params = predicate.params;
        if let Some(access) = target_access {
            let access_sql = super::base::resolve_where_clauses(
                self.schema,
                &target.object.name_singular,
                &target_alias,
                &access.clauses,
            )?;
            conditions.push(access_sql);
            params.extend(access.params);
        }
        Ok(CompiledFilter {
            clauses: vec![WhereClause::sql(
                JoinOp::And,
                relation_exists_sql(
                    &target.shape.schema_name,
                    &target.shape.table_name,
                    &target_alias,
                    conditions,
                )?,
            )],
            params,
        })
    }

    fn compile_value_filter(
        &mut self,
        object_name: &str,
        alias: &str,
        filter: Value,
    ) -> Result<CompiledFilter, RunnerError> {
        let schema = AliasedFilterSchema {
            schema: self.schema,
            object_name,
        };
        compile_filter(&filter, alias, &schema, self.suffixes)
            .map_err(RunnerError::from_filter)
            .map(|filter| filter.unwrap_or_else(unsatisfiable_filter))
    }
}

fn relation_exists_sql(
    schema_name: &str,
    table_name: &str,
    target_alias: &str,
    conditions: Vec<String>,
) -> Result<String, RunnerError> {
    Ok(format!(
        "EXISTS (SELECT 1 FROM {}.{} AS {} WHERE {})",
        quote_identifier(schema_name)?,
        quote_identifier(table_name)?,
        quote_identifier(target_alias)?,
        conditions.join(" AND ")
    ))
}

fn combine_compiled_filters(operator: JoinOp, filters: Vec<CompiledFilter>) -> CompiledFilter {
    if filters.is_empty() {
        return unsatisfiable_filter();
    }
    if filters.len() == 1 {
        return filters.into_iter().next().expect("one filter");
    }
    let mut clauses = Vec::with_capacity(filters.len());
    let mut params = Vec::new();
    for (index, filter) in filters.into_iter().enumerate() {
        if filter.clauses.len() == 1 {
            let mut clause = filter.clauses.into_iter().next().expect("one clause");
            clause.operator = if index == 0 { JoinOp::And } else { operator };
            clauses.push(clause);
        } else {
            clauses.push(WhereClause {
                operator: if index == 0 { JoinOp::And } else { operator },
                kind: crate::query::filter::ClauseKind::Group {
                    negated: false,
                    clauses: filter.clauses,
                },
            });
        }
        params.extend(filter.params);
    }
    CompiledFilter {
        clauses: vec![WhereClause::group(JoinOp::And, false, clauses)],
        params,
    }
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
    validate_rows(executor, schema, object_name_singular, ids, &access, false).await
}

pub(super) async fn validate_updated_rows<'e, E>(
    executor: E,
    schema: &RunnerSchema,
    object_name_singular: &str,
    ids: &[Uuid],
    access: &CompiledFilter,
) -> Result<(), RunnerError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    validate_rows(executor, schema, object_name_singular, ids, access, true).await
}

async fn validate_rows<'e, E>(
    executor: E,
    schema: &RunnerSchema,
    object_name_singular: &str,
    ids: &[Uuid],
    access: &CompiledFilter,
    updated: bool,
) -> Result<(), RunnerError>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    if ids.is_empty() {
        return Ok(());
    }
    let model = schema.object(object_name_singular)?;
    let alias = quote_identifier(object_name_singular)?;
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
    parameters.extend(access.params.iter().cloned());
    let sql = format!(
        "SELECT {} FROM {}.{} AS {} WHERE {}.{} = ANY(ARRAY[:...ids]::uuid[]) AND ({condition})",
        quote_identifier("id")?,
        quote_identifier(&model.shape.schema_name)?,
        quote_identifier(&model.shape.table_name)?,
        alias,
        alias,
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
            if updated {
                "Updated record does not satisfy row-level security constraints of your current role"
            } else {
                "Record does not satisfy row-level security constraints of your current role"
            },
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

fn operand_supported(
    field_type: FieldMetadataType,
    operand: &str,
    sub_field_name: Option<&str>,
) -> bool {
    let empty = matches!(operand, "IS_EMPTY" | "IS_NOT_EMPTY");
    match field_type {
        FieldMetadataType::Boolean => operand == "IS",
        FieldMetadataType::TsVector => operand == "VECTOR_SEARCH",
        FieldMetadataType::Uuid
        | FieldMetadataType::Relation
        | FieldMetadataType::MorphRelation
        | FieldMetadataType::Select => matches!(operand, "IS" | "IS_NOT") || empty,
        FieldMetadataType::Number | FieldMetadataType::Numeric | FieldMetadataType::Rating => {
            matches!(
                operand,
                "IS" | "IS_NOT" | "GREATER_THAN_OR_EQUAL" | "LESS_THAN_OR_EQUAL"
            ) || empty
        }
        FieldMetadataType::Date | FieldMetadataType::DateTime => {
            matches!(
                operand,
                "IS" | "IS_RELATIVE"
                    | "IS_IN_PAST"
                    | "IS_IN_FUTURE"
                    | "IS_TODAY"
                    | "IS_BEFORE"
                    | "IS_AFTER"
            ) || empty
        }
        FieldMetadataType::Currency => match sub_field_name {
            Some("currencyCode") => matches!(operand, "IS" | "IS_NOT") || empty,
            Some("amountMicros") => {
                matches!(
                    operand,
                    "IS" | "IS_NOT" | "GREATER_THAN_OR_EQUAL" | "LESS_THAN_OR_EQUAL"
                ) || empty
            }
            None => matches!(operand, "GREATER_THAN_OR_EQUAL" | "LESS_THAN_OR_EQUAL") || empty,
            Some(_) => false,
        },
        FieldMetadataType::Actor
            if matches!(sub_field_name, Some("source" | "workspaceMemberId")) =>
        {
            matches!(operand, "IS" | "IS_NOT") || empty
        }
        FieldMetadataType::Actor
        | FieldMetadataType::Address
        | FieldMetadataType::Array
        | FieldMetadataType::Emails
        | FieldMetadataType::Files
        | FieldMetadataType::FullName
        | FieldMetadataType::Links
        | FieldMetadataType::MultiSelect
        | FieldMetadataType::Phones
        | FieldMetadataType::RawJson => matches!(operand, "CONTAINS" | "DOES_NOT_CONTAIN") || empty,
        FieldMetadataType::Text | FieldMetadataType::RichText | FieldMetadataType::Position => {
            matches!(operand, "IS" | "IS_NOT" | "CONTAINS" | "DOES_NOT_CONTAIN") || empty
        }
    }
}

fn operand_filter(
    field_type: FieldMetadataType,
    operand: &str,
    sub_field_name: Option<&str>,
    value: Value,
) -> Result<Value, RunnerError> {
    if !operand_supported(field_type, operand, sub_field_name) {
        return Err(invalid(format!(
            "Operand {operand} is not supported for {} row-level predicates",
            field_type.as_str()
        )));
    }
    if matches!(operand, "IS_IN_PAST" | "IS_IN_FUTURE" | "IS_TODAY") {
        let now = chrono::Utc::now();
        return match field_type {
            FieldMetadataType::Date => {
                let today = now.format("%Y-%m-%d").to_string();
                Ok(match operand {
                    "IS_IN_PAST" => json!({ "lt": today }),
                    "IS_IN_FUTURE" => json!({ "gte": today }),
                    _ => json!({ "eq": today }),
                })
            }
            FieldMetadataType::DateTime => {
                let rounded = chrono::DateTime::from_timestamp((now.timestamp() + 30) / 60 * 60, 0)
                    .ok_or_else(|| invalid("Cannot resolve current date-time"))?;
                let rounded = rounded.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                Ok(match operand {
                    "IS_IN_PAST" => json!({ "lt": rounded }),
                    "IS_IN_FUTURE" => json!({ "gt": rounded }),
                    _ => {
                        let start = now
                            .date_naive()
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| invalid("Cannot resolve start of day"))?
                            .and_utc();
                        let end = start + chrono::Duration::days(1);
                        json!({
                            "and": [
                                { "gte": start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true) },
                                { "lt": end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true) }
                            ]
                        })
                    }
                })
            }
            _ => Err(invalid(format!(
                "Operand {operand} is not supported for {} row-level predicates",
                field_type.as_str()
            ))),
        };
    }
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
            | FieldMetadataType::MorphRelation
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

fn morph_filter_value(value: Value) -> Result<Value, RunnerError> {
    if let Some(selected) = value.get("selectedRecordIds") {
        return Ok(as_array(selected.clone()));
    }
    if value.is_object() {
        return Err(invalid(
            "Morph relation row-level predicates require selectedRecordIds",
        ));
    }
    Ok(value)
}

fn morph_relation_filter(
    model: &ObjectModel,
    field: &FlatFieldMetadata,
    raw_filter: Value,
    include_siblings: bool,
) -> Result<Value, RunnerError> {
    let is_head =
        field.morph_id.map(|id| id.to_string()) == Some(field.universal_identifier.clone());
    if !include_siblings && !is_head {
        let join_column = model
            .shape
            .relation(&field.name)
            .and_then(|relation| relation.join_column_name.clone())
            .ok_or_else(|| {
                invalid("Morph relation join column not found for row-level predicate")
            })?;
        return Ok(json!({ (join_column): raw_filter }));
    }
    let morph_id = field
        .morph_id
        .ok_or_else(|| invalid("Morph relation metadata not found for row-level predicate"))?;
    let mut join_columns = model
        .fields_by_name
        .values()
        .filter(|candidate| {
            candidate.field_type == FieldMetadataType::MorphRelation
                && candidate.morph_id == Some(morph_id)
        })
        .filter_map(|candidate| {
            model
                .shape
                .relation(&candidate.name)
                .and_then(|relation| relation.join_column_name.clone())
        })
        .collect::<Vec<_>>();
    join_columns.sort();
    join_columns.dedup();
    if join_columns.is_empty() {
        return Err(invalid(
            "Morph relation join columns not found for row-level predicate",
        ));
    }
    if join_columns.len() == 1 {
        return Ok(json!({ (join_columns[0].clone()): raw_filter }));
    }
    Ok(json!({
        "or": join_columns
            .into_iter()
            .map(|column| json!({ (column): raw_filter.clone() }))
            .collect::<Vec<_>>()
    }))
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

fn with_relative_field(mut filter: Value, field_name: &str) -> Value {
    if let Some(branches) = filter.get_mut("and").and_then(Value::as_array_mut) {
        for branch in branches {
            if let Some(operator) = branch.as_object().cloned() {
                *branch = json!({ (field_name): operator });
            }
        }
    }
    filter
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
        let filter = combine_compiled_filters(
            JoinOp::Or,
            vec![
                CompiledFilter {
                    clauses: vec![WhereClause::sql(JoinOp::And, "status = :status")],
                    params: vec![(String::from("status"), json!("OPEN"))],
                },
                CompiledFilter {
                    clauses: vec![WhereClause::sql(JoinOp::And, "priority >= :priority")],
                    params: vec![(String::from("priority"), json!(3))],
                },
            ],
        );
        assert_eq!(
            filter.render(),
            "((status = :status) OR (priority >= :priority))"
        );
        assert_eq!(
            filter.params,
            vec![
                (String::from("status"), json!("OPEN")),
                (String::from("priority"), json!(3))
            ]
        );
        assert_eq!(
            combine_compiled_filters(JoinOp::And, Vec::new()).render(),
            "(FALSE)"
        );
    }

    #[test]
    fn relation_exists_keeps_target_scope_in_the_same_query() {
        let sql = relation_exists_sql(
            "workspace",
            "company",
            "rls_0000000001",
            vec![
                String::from("\"rls_0000000001\".\"id\" = \"person\".\"companyId\""),
                String::from("\"rls_0000000001\".\"deletedAt\" IS NULL"),
                String::from("(\"rls_0000000001\".\"name\" = :name0000000002)"),
                String::from("(\"rls_0000000001\".\"domainName\" IS NOT NULL)"),
            ],
        )
        .unwrap();
        assert!(sql.contains("FROM \"workspace\".\"company\" AS \"rls_0000000001\""));
        assert!(sql.contains("AND (\"rls_0000000001\".\"domainName\" IS NOT NULL)"));
        assert!(sql.contains("\"rls_0000000001\".\"id\" = \"person\".\"companyId\""));
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

    #[tokio::test]
    async fn morph_operands_use_selected_ids_and_the_correct_join_columns() {
        let workspace_id = Uuid::new_v4();
        let object_id = Uuid::new_v4();
        let morph_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4().to_string();
        let mut head = field(object_id, "owner");
        head.field_type = FieldMetadataType::MorphRelation;
        head.universal_identifier = morph_id.to_string();
        head.morph_id = Some(morph_id);
        head.settings = Some(json!({"relationType": "MANY_TO_ONE"}));
        let mut company = field(object_id, "ownerCompany");
        company.field_type = FieldMetadataType::MorphRelation;
        company.morph_id = Some(morph_id);
        company.settings = Some(json!({"relationType": "MANY_TO_ONE"}));
        let mut person = field(object_id, "ownerPerson");
        person.field_type = FieldMetadataType::MorphRelation;
        person.morph_id = Some(morph_id);
        person.settings = Some(json!({"relationType": "MANY_TO_ONE"}));
        let fields = vec![
            field(object_id, "id"),
            head.clone(),
            company.clone(),
            person.clone(),
        ];
        let mut field_maps = crate::metadata::flat::FlatEntityMaps::default();
        for field in &fields {
            field_maps.add(
                field.id.to_string(),
                field.clone(),
                field.id.to_string(),
                workspace_id.to_string(),
            );
        }
        let object = object(workspace_id, object_id, "item", &fields);
        let schema = RunnerSchema::build(
            &workspace_id,
            std::slice::from_ref(&object),
            &field_maps,
            &crate::metadata::flat::FlatEntityMaps::default(),
        )
        .expect("morph schema");
        let model = schema.object("item").expect("morph model");
        let raw = json!({"in": [owner_id]});
        let expected = json!({
            "or": [
                {"ownerCompanyId": {"in": [owner_id]}},
                {"ownerPersonId": {"in": [owner_id]}}
            ]
        });

        assert_eq!(
            morph_filter_value(json!({"selectedRecordIds": [owner_id]})).unwrap(),
            json!([owner_id])
        );
        assert_eq!(
            operand_filter(
                FieldMetadataType::MorphRelation,
                "IS",
                None,
                json!([owner_id])
            )
            .unwrap(),
            json!({"in": [owner_id]})
        );
        assert_eq!(
            morph_relation_filter(model, &head, raw.clone(), true).unwrap(),
            expected
        );
        assert_eq!(
            morph_relation_filter(model, &company, raw.clone(), true).unwrap(),
            expected
        );
        assert_eq!(
            morph_relation_filter(model, &company, json!({"is": "NULL"}), false).unwrap(),
            json!({"ownerCompanyId": {"is": "NULL"}})
        );
        assert_eq!(
            morph_relation_filter(model, &company, json!({"is": "NOT_NULL"}), false).unwrap(),
            json!({"ownerCompanyId": {"is": "NOT_NULL"}})
        );

        let predicate = json!({
            "id": Uuid::new_v4().to_string(),
            "fieldMetadataId": company.id.to_string(),
            "objectMetadataId": object_id.to_string(),
            "operand": "IS",
            "value": {"selectedRecordIds": [owner_id]}
        });
        let mut permissions = crate::permissions::RoleRecordsPermissions {
            objects: [(object_id.to_string(), Default::default())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let object_permission = permissions
            .objects
            .get_mut(&object_id.to_string())
            .expect("object permission");
        object_permission.can_read_some_records = true;
        object_permission.row_level_records_permissions = Some(vec![rule(vec![predicate])]);
        let schema = RunnerSchema::build_with_permissions(
            &workspace_id,
            std::slice::from_ref(&object),
            &field_maps,
            &crate::metadata::flat::FlatEntityMaps::default(),
            permissions,
            None,
        )
        .expect("permission schema");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/nudge")
            .expect("lazy morph pool");
        let filter = row_access_filter(
            &pool,
            &schema,
            "item",
            ObjectRecordsOperationType::Read,
            &mut Suffixes::new(),
        )
        .await
        .expect("morph access filter")
        .expect("morph rule");
        let sql = filter.render();

        assert!(sql.contains("ownerCompanyId"), "{sql}");
        assert!(sql.contains("ownerPersonId"), "{sql}");
        assert!(sql.contains(" OR "), "{sql}");
        assert!(filter
            .params
            .iter()
            .any(|(_, value)| value.to_string().contains(&owner_id)));

        if let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") {
            use sqlx::Row;

            let _guard = crate::query::runner::test_support::DB_TEST_LOCK
                .lock()
                .await;
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&url)
                .await
                .expect("morph test database");
            let schema_name = nudge_core::ids::workspace_schema_name(&Uuid::new_v4());
            let schema_sql = quote_identifier(&schema_name).expect("schema identifier");
            let table_sql = format!(
                "{}.{}",
                schema_sql,
                quote_identifier("item").expect("table")
            );
            sqlx::query(&format!("CREATE SCHEMA {}", schema_sql))
                .execute(&pool)
                .await
                .expect("create morph schema");
            sqlx::query(&format!(
                "CREATE TABLE {} (id uuid PRIMARY KEY, \"ownerCompanyId\" uuid, \"ownerPersonId\" uuid)",
                table_sql
            ))
            .execute(&pool)
            .await
            .expect("create morph table");
            let owner_uuid = Uuid::parse_str(&owner_id).expect("owner uuid");
            let company_match = Uuid::new_v4();
            let person_match = Uuid::new_v4();
            let no_match = Uuid::new_v4();
            for (id, company_id, person_id) in [
                (company_match, Some(owner_uuid), None),
                (person_match, None, Some(owner_uuid)),
                (no_match, None, None),
            ] {
                sqlx::query(&format!(
                    "INSERT INTO {} (id, \"ownerCompanyId\", \"ownerPersonId\") VALUES ($1, $2, $3)",
                    table_sql
                ))
                .bind(id)
                .bind(company_id)
                .bind(person_id)
                .execute(&pool)
                .await
                .expect("insert morph row");
            }
            let parameters = filter.params.into_iter().collect::<Map<_, _>>();
            let compiled = crate::query::compile::compile_named_parameters(
                &format!("SELECT id FROM {} WHERE {}", table_sql, sql),
                &parameters,
            )
            .expect("compile morph query");
            let mut query = sqlx::query(&compiled.text);
            for value in &compiled.values {
                query =
                    super::super::base::bind_json_value(query, value).expect("bind morph value");
            }
            let rows = query.fetch_all(&pool).await.expect("query morph rows");
            let mut ids = rows
                .iter()
                .map(|row| row.try_get::<Uuid, _>(0).expect("morph row id"))
                .collect::<Vec<_>>();
            ids.sort();
            sqlx::query(&format!("DROP SCHEMA {} CASCADE", schema_sql))
                .execute(&pool)
                .await
                .expect("drop morph schema");
            let mut expected = vec![company_match, person_match];
            expected.sort();
            assert_eq!(ids, expected);
        }
    }

    #[test]
    fn text_predicates_reject_upstream_unsupported_operands() {
        for operand in ["IS_NOT_NULL", "GREATER_THAN_OR_EQUAL", "IS_BEFORE"] {
            assert!(!operand_supported(FieldMetadataType::Text, operand, None));
        }
        assert!(operand_supported(
            FieldMetadataType::Text,
            "IS_NOT_EMPTY",
            None
        ));
        assert!(operand_supported(
            FieldMetadataType::Number,
            "GREATER_THAN_OR_EQUAL",
            None
        ));
    }

    #[test]
    fn value_less_date_operands_match_upstream_boundaries() {
        let past =
            operand_filter(FieldMetadataType::Date, "IS_IN_PAST", None, Value::Null).unwrap();
        assert_eq!(past.as_object().unwrap().len(), 1);
        assert!(past["lt"].as_str().is_some_and(|value| value.len() == 10));

        let future =
            operand_filter(FieldMetadataType::Date, "IS_IN_FUTURE", None, Value::Null).unwrap();
        assert!(future["gte"]
            .as_str()
            .is_some_and(|value| value.len() == 10));

        let today =
            operand_filter(FieldMetadataType::DateTime, "IS_TODAY", None, Value::Null).unwrap();
        assert_eq!(today["and"].as_array().unwrap().len(), 2);
        assert!(today["and"][0].get("gte").is_some());
        assert!(today["and"][1].get("lt").is_some());

        let future = operand_filter(
            FieldMetadataType::DateTime,
            "IS_IN_FUTURE",
            None,
            Value::Null,
        )
        .unwrap();
        assert!(future.get("gt").is_some());
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

    #[tokio::test]
    async fn relative_date_predicate_compiles_to_a_top_level_range() {
        let workspace_id = Uuid::new_v4();
        let object_id = Uuid::new_v4();
        let mut due_at = field(object_id, "dueAt");
        due_at.field_type = FieldMetadataType::Date;
        let fields = vec![field(object_id, "id"), due_at.clone()];
        let mut field_maps = crate::metadata::flat::FlatEntityMaps::default();

        for item in &fields {
            field_maps.add(
                item.id.to_string(),
                item.clone(),
                item.id.to_string(),
                workspace_id.to_string(),
            );
        }

        let object = object(workspace_id, object_id, "event", &fields);
        let mut permissions = crate::permissions::RoleRecordsPermissions {
            objects: [(object_id.to_string(), Default::default())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let object_permission = permissions
            .objects
            .get_mut(&object_id.to_string())
            .expect("object permission");
        object_permission.object_metadata_id = object_id.to_string();
        object_permission.row_level_records_permissions = Some(vec![rule(vec![json!({
            "id": Uuid::new_v4().to_string(),
            "fieldMetadataId": due_at.id.to_string(),
            "objectMetadataId": object_id.to_string(),
            "operand": "IS_RELATIVE",
            "value": "THIS_1_DAY;;UTC;;"
        })])]);
        object_permission.can_read_some_records = true;
        let schema = RunnerSchema::build_with_permissions(
            &workspace_id,
            &[object],
            &field_maps,
            &crate::metadata::flat::FlatEntityMaps::default(),
            permissions,
            None,
        )
        .expect("relative date schema");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/nudge")
            .expect("lazy relative date pool");
        let access = row_access_filter(
            &pool,
            &schema,
            "event",
            ObjectRecordsOperationType::Read,
            &mut Suffixes::new(),
        )
        .await
        .expect("relative date access")
        .expect("relative date filter");
        let sql = access.render();

        assert!(sql.contains("\"event\".\"dueAt\" >="), "{sql}");
        assert!(sql.contains("\"event\".\"dueAt\" <"), "{sql}");
        assert!(!sql.contains("\"dueAt\".\"and\""), "{sql}");
        assert_eq!(access.params.len(), 2);
        assert!(access.params.iter().all(|(_, value)| value.is_string()));
    }

    #[tokio::test]
    async fn relation_hop_applies_target_read_scope_for_to_one() {
        let (pool, schema) = relation_schema("person", "company", "company", true, false);
        let access = row_access_filter(
            &pool,
            &schema,
            "person",
            ObjectRecordsOperationType::Read,
            &mut Suffixes::new(),
        )
        .await
        .expect("to-one relation access")
        .expect("to-one relation filter");

        assert!(access
            .render()
            .contains("\"rls_0000000001\".\"id\" = \"person\".\"companyId\""));
        assert!(access
            .render()
            .contains("\"rls_0000000001\".\"name\" = :name0000000002"));
        assert!(
            access
                .render()
                .contains("\"rls_0000000001\".\"status\" = :status0000000004"),
            "{} {:?}",
            access.render(),
            access.params
        );
        assert_eq!(access.params.len(), 2);
        assert_eq!(access.params[0].1, json!("Visible"));
        assert_eq!(access.params[1].1, json!("ACTIVE"));
    }

    #[tokio::test]
    async fn relation_hop_applies_target_read_scope_for_to_many() {
        let (pool, schema) = relation_schema("company", "person", "people", true, false);
        let access = row_access_filter(
            &pool,
            &schema,
            "company",
            ObjectRecordsOperationType::Read,
            &mut Suffixes::new(),
        )
        .await
        .expect("to-many relation access")
        .expect("to-many relation filter");

        assert!(access
            .render()
            .contains("\"rls_0000000001\".\"companyId\" = \"company\".\"id\""));
        assert!(access
            .render()
            .contains("\"rls_0000000001\".\"name\" = :name0000000002"));
        assert!(
            access
                .render()
                .contains("\"rls_0000000001\".\"status\" = :status0000000004"),
            "{} {:?}",
            access.render(),
            access.params
        );
        assert_eq!(access.params.len(), 2);
        assert_ne!(access.params[0].0, access.params[1].0);
    }

    #[tokio::test]
    async fn relation_hop_is_denied_until_both_feature_flags_are_enabled() {
        let (pool, schema) = relation_schema("person", "company", "company", false, false);
        let access = row_access_filter(
            &pool,
            &schema,
            "person",
            ObjectRecordsOperationType::Read,
            &mut Suffixes::new(),
        )
        .await
        .expect("disabled relation access")
        .expect("disabled relation filter");

        assert_eq!(access.render(), "(FALSE)");
    }

    #[tokio::test]
    async fn member_relation_hops_are_denied_when_the_feature_is_disabled() {
        let (pool, schema) = relation_schema("person", "company", "company", false, true);
        let access = row_access_filter(
            &pool,
            &schema,
            "person",
            ObjectRecordsOperationType::Read,
            &mut Suffixes::new(),
        )
        .await
        .expect("disabled member relation access")
        .expect("disabled member relation filter");

        assert_eq!(access.render(), "(FALSE)");
    }

    fn relation_schema(
        source_name: &str,
        target_name: &str,
        relation_name: &str,
        relation_hop_enabled: bool,
        member_relation: bool,
    ) -> (PgPool, RunnerSchema) {
        let workspace_id = Uuid::new_v4();
        let person_id = Uuid::new_v4();
        let company_id = Uuid::new_v4();
        let person_name = field(person_id, "name");
        let company_name = field(company_id, "name");
        let person_status = field(person_id, "status");
        let company_status = field(company_id, "status");
        let mut company = relation_field(person_id, "company", company_id);
        let mut people = relation_field(company_id, "people", person_id);
        company.relation_target_field_metadata_id = Some(people.id);
        people.relation_target_field_metadata_id = Some(company.id);
        company.settings = Some(json!({ "relationType": "MANY_TO_ONE" }));
        people.settings = Some(json!({ "relationType": "ONE_TO_MANY" }));
        let person_fields = vec![
            field(person_id, "id"),
            person_name.clone(),
            person_status.clone(),
            company.clone(),
        ];
        let company_fields = vec![
            field(company_id, "id"),
            company_name.clone(),
            company_status.clone(),
            people.clone(),
        ];
        let mut fields = crate::metadata::flat::FlatEntityMaps::default();

        for item in person_fields.iter().chain(&company_fields) {
            fields.add(
                item.id.to_string(),
                item.clone(),
                item.id.to_string(),
                workspace_id.to_string(),
            );
        }

        let person = object(workspace_id, person_id, "person", &person_fields);
        let company_object = object(workspace_id, company_id, "company", &company_fields);
        let (source_id, target_id, relation_field_id, name_field_id, status_field_id) =
            if relation_name == "company" {
                (
                    person_id,
                    company_id,
                    company.id,
                    company_name.id,
                    company_status.id,
                )
            } else {
                (
                    company_id,
                    person_id,
                    people.id,
                    person_name.id,
                    person_status.id,
                )
            };
        let mut relation_predicate = json!({
            "id": Uuid::new_v4().to_string(),
            "fieldMetadataId": name_field_id.to_string(),
            "relationFieldMetadataId": relation_field_id.to_string(),
            "objectMetadataId": source_id.to_string(),
            "operand": "IS",
            "value": "Visible"
        });
        if member_relation {
            relation_predicate["workspaceMemberRelationFieldMetadataId"] =
                Value::String(relation_field_id.to_string());
        }
        let relation_rule = rule(vec![relation_predicate]);
        let target_rule = rule(vec![json!({
            "id": Uuid::new_v4().to_string(),
            "fieldMetadataId": status_field_id.to_string(),
            "objectMetadataId": target_id.to_string(),
            "operand": "IS",
            "value": "ACTIVE"
        })]);
        let mut permissions = crate::permissions::RoleRecordsPermissions {
            objects: [
                (source_id.to_string(), Default::default()),
                (target_id.to_string(), Default::default()),
            ]
            .into_iter()
            .collect(),
            relation_hop_enabled,
            ..Default::default()
        };
        let source_permission = permissions
            .objects
            .get_mut(&source_id.to_string())
            .expect("source permission");
        source_permission.object_metadata_id = source_id.to_string();
        source_permission.row_level_records_permissions = Some(vec![relation_rule]);
        source_permission.can_read_some_records = true;
        let target_permission = permissions
            .objects
            .get_mut(&target_id.to_string())
            .expect("target permission");
        target_permission.object_metadata_id = target_id.to_string();
        target_permission.row_level_records_permissions = Some(vec![target_rule]);
        target_permission.can_read_some_records = true;
        let schema = RunnerSchema::build_with_permissions(
            &workspace_id,
            &[person, company_object],
            &fields,
            &crate::metadata::flat::FlatEntityMaps::default(),
            permissions,
            None,
        )
        .expect("relation permission schema");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/nudge")
            .expect("lazy relation pool");

        assert_eq!(
            schema.object(source_name).expect("source").object.id,
            source_id
        );
        assert_eq!(
            schema.object(target_name).expect("target").object.id,
            target_id
        );
        (pool, schema)
    }

    fn object(
        workspace_id: Uuid,
        id: Uuid,
        name: &str,
        fields: &[crate::metadata::flat::FlatFieldMetadata],
    ) -> crate::metadata::flat::FlatObjectMetadata {
        crate::metadata::flat::FlatObjectMetadata {
            id,
            workspace_id,
            name_singular: name.to_string(),
            name_plural: format!("{name}s"),
            label_singular: name.to_string(),
            label_plural: format!("{name}s"),
            target_table_name: Some(name.to_string()),
            field_ids: fields.iter().map(|field| field.id.to_string()).collect(),
            ..Default::default()
        }
    }

    fn field(object_id: Uuid, name: &str) -> crate::metadata::flat::FlatFieldMetadata {
        crate::metadata::flat::FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: if name == "id" {
                FieldMetadataType::Uuid
            } else {
                FieldMetadataType::Text
            },
            name: name.to_string(),
            label: name.to_string(),
            ..Default::default()
        }
    }

    fn relation_field(
        object_id: Uuid,
        name: &str,
        target_object_id: Uuid,
    ) -> crate::metadata::flat::FlatFieldMetadata {
        crate::metadata::flat::FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: FieldMetadataType::Relation,
            name: name.to_string(),
            label: name.to_string(),
            relation_target_object_metadata_id: Some(target_object_id),
            ..Default::default()
        }
    }

    fn rule(predicates: Vec<Value>) -> RowLevelRecordsPermissions {
        RowLevelRecordsPermissions {
            rule_id: Uuid::new_v4().to_string(),
            can_read_records: true,
            predicates,
            ..Default::default()
        }
    }
}
