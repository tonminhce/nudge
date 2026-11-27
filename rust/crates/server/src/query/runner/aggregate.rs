use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;
use crate::metadata::names::compute_composite_column_name;
use crate::query::filter::Suffixes;

use super::base::compile_runner_filter;
use super::base::execute_json_rows;
use super::base::resolve_where_clauses;
use super::base::RunnerSchema;
use super::find_many::collect_filter_relation_joins;
use super::find_many::from_with_joins;
use super::find_many::RelationJoins;
use super::permissions::validate_filter_read;
use super::permissions::validate_named_read;
use super::RunnerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateOperation {
    TotalCount,
    CountEmpty,
    CountNotEmpty,
    CountUniqueValues,
    PercentageEmpty,
    PercentageNotEmpty,
    CountTrue,
    CountFalse,
    Min,
    Max,
    Average,
    Sum,
}

impl AggregateOperation {
    pub fn parse(value: &str) -> Option<Self> {
        let operation = match value {
            "totalCount" => Self::TotalCount,
            "countEmpty" => Self::CountEmpty,
            "countNotEmpty" => Self::CountNotEmpty,
            "countUniqueValues" => Self::CountUniqueValues,
            "percentageEmpty" => Self::PercentageEmpty,
            "percentageNotEmpty" => Self::PercentageNotEmpty,
            "countTrue" => Self::CountTrue,
            "countFalse" => Self::CountFalse,
            "min" => Self::Min,
            "max" => Self::Max,
            "avg" => Self::Average,
            "sum" => Self::Sum,
            _ => return None,
        };

        Some(operation)
    }

    pub fn prefix(self) -> &'static str {
        match self {
            Self::TotalCount => "totalCount",
            Self::CountEmpty => "countEmpty",
            Self::CountNotEmpty => "countNotEmpty",
            Self::CountUniqueValues => "countUniqueValues",
            Self::PercentageEmpty => "percentageEmpty",
            Self::PercentageNotEmpty => "percentageNotEmpty",
            Self::CountTrue => "countTrue",
            Self::CountFalse => "countFalse",
            Self::Min => "min",
            Self::Max => "max",
            Self::Average => "avg",
            Self::Sum => "sum",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aggregate {
    pub operation: AggregateOperation,
    pub field_name: Option<String>,
    pub sub_field_name: Option<String>,
}

impl Aggregate {
    pub fn total_count() -> Self {
        Self {
            operation: AggregateOperation::TotalCount,
            field_name: None,
            sub_field_name: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateValue {
    pub name: String,
    pub value: Value,
}

pub async fn aggregate(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    filter: Option<&Value>,
    with_deleted: bool,
    aggregates: &[Aggregate],
) -> Result<Vec<AggregateValue>, RunnerError> {
    let model = schema.object(object_name_singular)?;
    let alias = object_name_singular;
    validate_filter_read(schema, model, filter)?;
    let field_names = aggregates
        .iter()
        .filter_map(|aggregate| aggregate.field_name.clone())
        .collect::<Vec<_>>();
    validate_named_read(schema, model, &field_names)?;
    let mut suffixes = Suffixes::new();
    let compiled_filter =
        compile_runner_filter(schema, object_name_singular, filter, &mut suffixes)?;
    let mut parameter_map = serde_json::Map::new();
    let clauses;

    if let Some(compiled_filter) = compiled_filter {
        clauses = compiled_filter.clauses;
        parameter_map.extend(compiled_filter.params);
    } else {
        clauses = Vec::new();
    }

    let mut joins = RelationJoins::default();
    collect_filter_relation_joins(schema, object_name_singular, alias, filter, &mut joins)?;
    let where_expression = resolve_where_clauses(schema, object_name_singular, alias, &clauses)?;
    let mut predicates = Vec::new();

    if !where_expression.is_empty() {
        predicates.push(where_expression);
    }

    if !with_deleted && model.shape.has_deleted_at_column {
        predicates.push(format!(
            "{}.{} IS NULL",
            crate::query::repository::quote_identifier(alias)?,
            crate::query::repository::quote_identifier("deletedAt")?
        ));
    }

    let where_sql = if predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", predicates.join(" AND "))
    };
    let from_sql = from_with_joins(model, alias, &joins)?;
    let mut expressions = Vec::with_capacity(aggregates.len());

    for aggregate in aggregates {
        let name = aggregate_result_name(aggregate);
        let expression = aggregate_expression(model, alias, aggregate)?;
        expressions.push((name, expression));
    }

    let select_sql = expressions
        .iter()
        .map(|(name, expression)| {
            Ok(format!(
                "{expression} AS {}",
                crate::query::repository::quote_identifier(&super::base::column_result_alias(
                    alias, name
                ))?
            ))
        })
        .collect::<Result<Vec<_>, RunnerError>>()?
        .join(", ");
    let sql = format!("SELECT {select_sql} FROM {from_sql}{where_sql}");
    let compiled_sql = crate::query::compile::compile_named_parameters(&sql, &parameter_map)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let rows = execute_json_rows(pool, &compiled_sql.text, &compiled_sql.values).await?;
    let row = rows.first().cloned().unwrap_or_default();

    Ok(expressions
        .into_iter()
        .map(|(name, _)| {
            let result_alias = super::base::column_result_alias(alias, &name);

            AggregateValue {
                name,
                value: row.get(&result_alias).cloned().unwrap_or(Value::Null),
            }
        })
        .collect())
}

pub fn aggregate_result_name(aggregate: &Aggregate) -> String {
    match aggregate.operation {
        AggregateOperation::TotalCount => String::from("totalCount"),
        operation => {
            let field_name = aggregate.field_name.as_deref().unwrap_or_default();
            format!("{}{}", operation.prefix(), capitalize(field_name))
        }
    }
}

pub fn aggregate_expression(
    model: &super::base::ObjectModel,
    alias: &str,
    aggregate: &Aggregate,
) -> Result<String, RunnerError> {
    let Some(field_name) = aggregate.field_name.as_deref() else {
        return Ok(String::from("count(*)::bigint"));
    };

    let (column_name, field_type) =
        physical_field(model, field_name, aggregate.sub_field_name.as_deref())?;
    let column = super::base::quoted_column(alias, &column_name)?;

    match aggregate.operation {
        AggregateOperation::TotalCount => Ok(String::from("count(*)::bigint")),
        AggregateOperation::CountEmpty => Ok(format!(
            "CASE WHEN count(*) = 0 THEN NULL ELSE count(*) - count({column}) END"
        )),
        AggregateOperation::CountNotEmpty => Ok(format!(
            "CASE WHEN count(*) = 0 THEN NULL ELSE count(NULLIF(CONCAT({column}), '')) END"
        )),
        AggregateOperation::CountUniqueValues => Ok(format!(
            "CASE WHEN count(*) = 0 THEN NULL ELSE count(DISTINCT {column}) END"
        )),
        AggregateOperation::PercentageEmpty => Ok(format!(
            "CASE WHEN count(*) = 0 THEN NULL ELSE CAST((count(*) - count({column})::decimal) / count(*) AS decimal) END"
        )),
        AggregateOperation::PercentageNotEmpty => Ok(format!(
            "CASE WHEN count(*) = 0 THEN NULL ELSE CAST(count({column})::decimal / count(*) AS decimal) END"
        )),
        AggregateOperation::CountTrue => Ok(format!(
            "CASE WHEN count(*) = 0 THEN NULL ELSE count(CASE WHEN {column}::boolean = TRUE THEN 1 END) END"
        )),
        AggregateOperation::CountFalse => Ok(format!(
            "CASE WHEN count(*) = 0 THEN NULL ELSE count(CASE WHEN {column}::boolean = FALSE THEN 1 END) END"
        )),
        AggregateOperation::Min | AggregateOperation::Max => Ok(format!(
            "{}({column})",
            if aggregate.operation == AggregateOperation::Min {
                "min"
            } else {
                "max"
            }
        )),
        AggregateOperation::Average | AggregateOperation::Sum => {
            if field_type == FieldMetadataType::Rating {
                let cast = format!("CAST(SPLIT_PART({column}::text, '_', 2) AS INTEGER)");
                return Ok(format!(
                    "{}({cast})",
                    if aggregate.operation == AggregateOperation::Average {
                        "avg"
                    } else {
                        "sum"
                    }
                ));
            }

            Ok(format!(
                "{}({column})",
                if aggregate.operation == AggregateOperation::Average {
                    "avg"
                } else {
                    "sum"
                }
            ))
        }
    }
}

pub fn physical_field(
    model: &super::base::ObjectModel,
    field_name: &str,
    sub_field_name: Option<&str>,
) -> Result<(String, FieldMetadataType), RunnerError> {
    if let Some(sub_field_name) = sub_field_name {
        let column_name = compute_composite_column_name(field_name, sub_field_name);
        let column = model.shape.column(&column_name).ok_or_else(|| {
            RunnerError::unknown_sub_field(field_name, sub_field_name, Vec::new())
        })?;

        return Ok((column.column_name.clone(), column.field_metadata_type));
    }

    if let Some(column) = model.shape.column(field_name) {
        if column.composite_parent_field_name.is_none() {
            return Ok((column.column_name.clone(), column.field_metadata_type));
        }
    }

    Err(RunnerError::unknown_column(
        field_name,
        &model.object.name_singular,
    ))
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::query::compile::compile_named_parameters;
    use crate::query::repository::build_insert_rows;
    use crate::query::repository::build_insert_statement;
    use crate::query::runner::group_by;
    use crate::query::runner::test_support;
    use crate::query::runner::Granularity;
    use crate::query::runner::GroupByDefinition;
    use serde_json::json;
    use uuid::Uuid;

    async fn insert_rows(
        state: &crate::state::AppState,
        shape: &crate::query::repository::table_shape::TableShape,
        sql: &str,
        parameters: &serde_json::Map<String, Value>,
        column_names: &[String],
    ) -> Vec<Uuid> {
        let compiled = compile_named_parameters(sql, parameters).expect("compiled insert");
        let mut query = sqlx::query_scalar::<_, Uuid>(&compiled.text);

        for (index, value) in compiled.values.iter().enumerate() {
            let column_name = &column_names[index % column_names.len()];
            let column_type = shape
                .column(column_name)
                .map(|column| column.field_metadata_type)
                .unwrap_or(crate::metadata::flat::FieldMetadataType::Text);

            if matches!(
                column_type,
                crate::metadata::flat::FieldMetadataType::Uuid
                    | crate::metadata::flat::FieldMetadataType::Relation
                    | crate::metadata::flat::FieldMetadataType::MorphRelation
            ) {
                let value = value
                    .as_str()
                    .and_then(|value| Uuid::parse_str(value).ok())
                    .expect("uuid insert value");
                query = query.bind(value);
            } else {
                query = query.bind(value.as_str().expect("text insert value").to_string());
            }
        }

        query
            .fetch_all(&state.core_pool)
            .await
            .expect("aggregate insert executes")
    }

    #[tokio::test]
    async fn aggregate_and_group_by_match_the_upstream_field_names_and_limits() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };

        let _guard = test_support::DB_TEST_LOCK.lock().await;
        let seeded = test_support::seeded_workspace(&url)
            .await
            .expect("seeded aggregate workspace");
        let schema = test_support::runner_schema(&seeded).await;
        let company_shape = schema.object("company").expect("company").shape.clone();
        let person_shape = schema.object("person").expect("person").shape.clone();

        let company_rows = build_insert_rows(
            &company_shape,
            &[test_support::record(r#"{"name": "Nudge Inc"}"#)],
        )
        .expect("company rows");
        let company_sql = build_insert_statement(
            &company_shape,
            &company_rows.column_names,
            &company_rows.rows,
            &["id".to_string()],
            false,
        )
        .expect("company sql");
        let company_ids = insert_rows(
            &seeded.state,
            &company_shape,
            &company_sql,
            &company_rows.parameters,
            &company_rows.column_names,
        )
        .await;
        let company_id = company_ids.first().copied().expect("company id");

        let raw_people = [
            r#"{"name": {"firstName": "Ada", "lastName": "Lovelace"}, "jobTitle": "Engineer", "companyId": "COMPANY_ID"}"#,
            r#"{"name": {"firstName": "Grace", "lastName": "Hopper"}, "jobTitle": "Admiral", "companyId": "COMPANY_ID"}"#,
            r#"{"name": {"firstName": "Alan", "lastName": "Turing"}, "jobTitle": "Mathematician"}"#,
        ];
        let person_rows = build_insert_rows(
            &person_shape,
            &raw_people
                .iter()
                .map(|json| test_support::record(json))
                .map(|mut row| {
                    if let Some(value) = row.get_mut("companyId") {
                        if value == &Value::String(String::from("COMPANY_ID")) {
                            *value = Value::String(company_id.to_string());
                        }
                    }

                    row
                })
                .collect::<Vec<_>>(),
        )
        .expect("person rows");
        let person_sql = build_insert_statement(
            &person_shape,
            &person_rows.column_names,
            &person_rows.rows,
            &["id".to_string()],
            false,
        )
        .expect("person sql");
        let person_ids = insert_rows(
            &seeded.state,
            &person_shape,
            &person_sql,
            &person_rows.parameters,
            &person_rows.column_names,
        )
        .await;
        assert_eq!(person_ids.len(), 3);

        let aggregates = aggregate(
            &seeded.state.core_pool,
            &schema,
            "company",
            None,
            false,
            &[
                Aggregate::total_count(),
                Aggregate {
                    operation: AggregateOperation::CountNotEmpty,
                    field_name: Some(String::from("name")),
                    sub_field_name: None,
                },
                Aggregate {
                    operation: AggregateOperation::Min,
                    field_name: Some(String::from("createdAt")),
                    sub_field_name: None,
                },
            ],
        )
        .await
        .expect("company aggregate");
        let by_name = aggregates
            .iter()
            .map(|value| (value.name.as_str(), value.value.clone()))
            .collect::<std::collections::HashMap<_, _>>();

        assert_eq!(by_name.get("totalCount"), Some(&json!(1)));
        assert_eq!(by_name.get("countNotEmptyName"), Some(&json!(1)));
        assert!(by_name
            .get("minCreatedAt")
            .is_some_and(|value| value.is_string()));

        let groups = group_by(
            &seeded.state.core_pool,
            &schema,
            "person",
            None,
            false,
            &[GroupByDefinition {
                field_name: String::from("jobTitle"),
                sub_field_name: None,
                granularity: Granularity::None,
            }],
            &[],
            None,
            None,
        )
        .await
        .expect("person group by");
        let keys = groups
            .iter()
            .map(|row| row.group_key.as_str().unwrap_or_default())
            .collect::<Vec<_>>();

        assert_eq!(keys, vec!["Admiral", "Engineer", "Mathematician"]);
        assert!(groups.iter().all(|row| row
            .values
            .last()
            .map(|(_, value)| value == &json!(1))
            .unwrap_or(false)));

        let missing_timezone = group_by(
            &seeded.state.core_pool,
            &schema,
            "person",
            None,
            false,
            &[GroupByDefinition {
                field_name: String::from("createdAt"),
                sub_field_name: None,
                granularity: Granularity::Day,
            }],
            &[],
            None,
            None,
        )
        .await
        .unwrap_err();
        assert_eq!(
            missing_timezone.code(),
            "MISSING_TIMEZONE_FOR_DATE_GROUP_BY"
        );

        let invalid_timezone = group_by(
            &seeded.state.core_pool,
            &schema,
            "person",
            None,
            false,
            &[GroupByDefinition {
                field_name: String::from("createdAt"),
                sub_field_name: None,
                granularity: Granularity::Day,
            }],
            &[],
            None,
            Some("Not/AZone"),
        )
        .await
        .unwrap_err();
        assert_eq!(invalid_timezone.code(), "INVALID_TIMEZONE");

        test_support::drop_workspace(&seeded).await;
    }
}
