use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;
use crate::query::filter::Suffixes;

use super::aggregate::aggregate_expression;
use super::aggregate::aggregate_result_name;
use super::aggregate::physical_field;
use super::aggregate::Aggregate;
use super::base::compile_runner_filter;
use super::base::execute_json_rows;
use super::base::quoted_column;
use super::base::resolve_where_clauses;
use super::base::RunnerSchema;
use super::find_many::collect_filter_relation_joins;
use super::find_many::from_with_joins;
use super::find_many::RelationJoins;
use super::permissions::validate_filter_read;
use super::permissions::validate_named_read;
use super::rls::row_access_filter;
use super::RunnerError;

pub const DEFAULT_NUMBER_OF_GROUPS_LIMIT: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    None,
    DayOfTheWeek,
    MonthOfTheYear,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl Granularity {
    pub fn parse(value: &str) -> Option<Self> {
        let granularity = match value {
            "NONE" => Self::None,
            "DAY_OF_THE_WEEK" => Self::DayOfTheWeek,
            "MONTH_OF_THE_YEAR" => Self::MonthOfTheYear,
            "DAY" => Self::Day,
            "WEEK" => Self::Week,
            "MONTH" => Self::Month,
            "QUARTER" => Self::Quarter,
            "YEAR" => Self::Year,
            _ => return None,
        };

        Some(granularity)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::DayOfTheWeek => "DAY_OF_THE_WEEK",
            Self::MonthOfTheYear => "MONTH_OF_THE_YEAR",
            Self::Day => "DAY",
            Self::Week => "WEEK",
            Self::Month => "MONTH",
            Self::Quarter => "QUARTER",
            Self::Year => "YEAR",
        }
    }

    pub fn sql_truncation(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Quarter => "quarter",
            Self::Year => "year",
            Self::None | Self::DayOfTheWeek | Self::MonthOfTheYear => "day",
        }
    }

    pub fn requires_timezone(self) -> bool {
        matches!(
            self,
            Self::Day | Self::Week | Self::Month | Self::Quarter | Self::Year
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupByDefinition {
    pub field_name: String,
    pub sub_field_name: Option<String>,
    pub granularity: Granularity,
}

#[derive(Debug, Clone)]
pub struct GroupByRow {
    pub group_key: Value,
    pub values: Vec<(String, Value)>,
}

#[allow(clippy::too_many_arguments)]
pub async fn group_by(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    filter: Option<&Value>,
    with_deleted: bool,
    definitions: &[GroupByDefinition],
    aggregates: &[Aggregate],
    limit: Option<usize>,
    timezone: Option<&str>,
) -> Result<Vec<GroupByRow>, RunnerError> {
    let model = schema.object(object_name_singular)?;
    let alias = object_name_singular;
    validate_filter_read(schema, model, filter)?;
    let field_names = definitions
        .iter()
        .map(|definition| definition.field_name.clone())
        .chain(
            aggregates
                .iter()
                .filter_map(|aggregate| aggregate.field_name.clone()),
        )
        .collect::<Vec<_>>();
    validate_named_read(schema, model, &field_names)?;
    let mut suffixes = Suffixes::new();
    let compiled_filter =
        compile_runner_filter(schema, object_name_singular, filter, &mut suffixes)?;
    let mut parameter_map = serde_json::Map::new();
    let mut clauses;

    if let Some(compiled_filter) = compiled_filter {
        clauses = compiled_filter.clauses;
        parameter_map.extend(compiled_filter.params);
    } else {
        clauses = Vec::new();
    }
    if let Some(access) = row_access_filter(
        pool,
        schema,
        object_name_singular,
        crate::permissions::ObjectRecordsOperationType::Read,
        &mut suffixes,
    )
    .await?
    {
        clauses.extend(access.clauses);
        parameter_map.extend(access.params);
    }

    for definition in definitions {
        validate_definition(pool, model, definition, timezone).await?;
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
    let mut expressions = Vec::new();

    for definition in definitions {
        let expression = group_expression(model, alias, definition, timezone)?;
        expressions.push(expression);
    }

    let mut select_parts = Vec::new();

    for expression in &expressions {
        select_parts.push(format!(
            "{expression} AS {}",
            crate::query::repository::quote_identifier(&super::base::column_result_alias(
                alias, "groupKey"
            ))?
        ));
    }

    for aggregate in aggregates {
        let name = aggregate_result_name(aggregate);
        let expression = aggregate_expression(model, alias, aggregate)?;
        select_parts.push(format!(
            "{expression} AS {}",
            crate::query::repository::quote_identifier(&super::base::column_result_alias(
                alias, &name
            ))?
        ));
    }

    let total_count_alias = super::base::column_result_alias(alias, "totalCount");
    select_parts.push(format!(
        "count(*)::bigint AS {}",
        crate::query::repository::quote_identifier(&total_count_alias)?
    ));
    let group_by_sql = if expressions.is_empty() {
        String::new()
    } else {
        format!(" GROUP BY {}", expressions.join(", "))
    };
    let limit = limit.unwrap_or(DEFAULT_NUMBER_OF_GROUPS_LIMIT).max(1);
    let order_by_alias = super::base::column_result_alias(alias, "groupKey");
    let sql = format!(
        "SELECT {} FROM {from_sql}{where_sql}{group_by_sql} ORDER BY {} ASC NULLS LAST LIMIT {limit}",
        select_parts.join(", "),
        crate::query::repository::quote_identifier(&order_by_alias)?
    );
    let compiled_sql = crate::query::compile::compile_named_parameters(&sql, &parameter_map)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let rows = execute_json_rows(pool, &compiled_sql.text, &compiled_sql.values).await?;
    let group_key_alias = super::base::column_result_alias(alias, "groupKey");

    Ok(rows
        .into_iter()
        .map(|row| {
            let mut values = Vec::new();

            for aggregate in aggregates {
                let name = aggregate_result_name(aggregate);
                let alias = super::base::column_result_alias(alias, &name);
                values.push((name, row.get(&alias).cloned().unwrap_or(Value::Null)));
            }

            values.push((
                String::from("totalCount"),
                row.get(&total_count_alias).cloned().unwrap_or(Value::Null),
            ));

            GroupByRow {
                group_key: row.get(&group_key_alias).cloned().unwrap_or(Value::Null),
                values,
            }
        })
        .collect())
}

pub fn group_expression(
    model: &super::base::ObjectModel,
    alias: &str,
    definition: &GroupByDefinition,
    timezone: Option<&str>,
) -> Result<String, RunnerError> {
    let (column_name, field_type) = physical_field(
        model,
        &definition.field_name,
        definition.sub_field_name.as_deref(),
    )?;
    let column = quoted_column(alias, &column_name)?;

    if field_type == FieldMetadataType::MultiSelect {
        return Ok(format!(
            "UNNEST(CASE WHEN CARDINALITY({column}) > 0 THEN {column} ELSE ARRAY[{column}[1]] END)::text"
        ));
    }

    if !matches!(
        field_type,
        FieldMetadataType::Date | FieldMetadataType::DateTime
    ) {
        return Ok(column);
    }

    match definition.granularity {
        Granularity::DayOfTheWeek => Ok(format!("TRIM(TO_CHAR({column}, 'TMDay'))")),
        Granularity::MonthOfTheYear => Ok(format!("TRIM(TO_CHAR({column}, 'TMMonth'))")),
        Granularity::None => Ok(column),
        Granularity::Day
        | Granularity::Week
        | Granularity::Month
        | Granularity::Quarter
        | Granularity::Year => {
            let timezone = timezone.ok_or_else(|| {
                RunnerError::new(
                    "MISSING_TIMEZONE_FOR_DATE_GROUP_BY",
                    "A timezone is required for this date group by",
                )
            })?;
            let truncated_column = if field_type == FieldMetadataType::DateTime {
                format!("{column} AT TIME ZONE '{}'", timezone.replace('\'', "''"))
            } else {
                column
            };
            let shifted = if definition.granularity == Granularity::Week {
                format!("({truncated_column} + interval '2 days')")
            } else {
                truncated_column
            };

            Ok(format!(
                "TO_CHAR(DATE_TRUNC('{}', {shifted}), 'YYYY-MM-DD')",
                definition.granularity.sql_truncation()
            ))
        }
    }
}

async fn validate_definition(
    pool: &sqlx::PgPool,
    model: &super::base::ObjectModel,
    definition: &GroupByDefinition,
    timezone: Option<&str>,
) -> Result<(), RunnerError> {
    let (_, field_type) = physical_field(
        model,
        &definition.field_name,
        definition.sub_field_name.as_deref(),
    )?;

    if field_type != FieldMetadataType::DateTime || !definition.granularity.requires_timezone() {
        return Ok(());
    }

    let Some(timezone) = timezone else {
        return Err(RunnerError::new(
            "MISSING_TIMEZONE_FOR_DATE_GROUP_BY",
            "A timezone is required for this date group by",
        ));
    };

    let valid =
        sqlx::query_scalar::<_, i32>("SELECT 1 FROM pg_timezone_names WHERE name = $1 LIMIT 1")
            .bind(timezone)
            .fetch_optional(pool)
            .await
            .map_err(RunnerError::database)?;

    if valid.is_none() {
        return Err(RunnerError::new(
            "INVALID_TIMEZONE",
            format!("Invalid timezone: {timezone}"),
        ));
    }

    Ok(())
}
