use serde_json::Map;
use serde_json::Value;

use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::RelationType;
use crate::query::cursor::build_cursor_data;
use crate::query::cursor::build_cursor_page;
use crate::query::cursor::compute_cursor_arg_filter;
use crate::query::cursor::get_cursor;
use crate::query::cursor::get_page_info;
use crate::query::cursor::merge_cursor_conditions;
use crate::query::cursor::CursorPaginationDirection;
use crate::query::cursor::LeafKind;
use crate::query::cursor::OrderByLeafSpec;
use crate::query::cursor::PageInfo;
use crate::query::filter::model::CompiledFilter;
use crate::query::filter::model::FilterSchema;
use crate::query::filter::Suffixes;
use crate::query::order::OrderByDirection;
use crate::query::order::DEFAULT_ID_ORDER_BY_TIEBREAKER;
use crate::query::repository::quote_identifier;

use super::base::compile_runner_filter;
use super::base::composite_column;
use super::base::direction;
use super::base::execute_json_rows;
use super::base::order_expression;
use super::base::render_select_list;
use super::base::resolve_where_clauses;
use super::base::select_columns;
use super::base::ResolvedOrder;
use super::base::RunnerSchema;
use super::base::Selection;
use super::base::DEFAULT_PAGE_SIZE;
use super::base::QUERY_MAX_RECORDS;
use super::format_result_rows;
use super::RunnerError;

#[derive(Debug, Clone, Default)]
pub struct FindManyArgs {
    pub filter: Option<Value>,
    pub order_by: Option<Value>,
    pub first: Option<usize>,
    pub last: Option<usize>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub offset: Option<i64>,
    pub with_deleted: bool,
    pub selected_field_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NestedRelationSelection {
    pub relation_field_name: String,
    pub selected_field_names: Vec<String>,
    pub first: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct FindManyResult {
    pub records: Vec<Value>,
    pub page_info: PageInfo,
    pub total_count: Option<i64>,
    pub cursors: std::collections::HashMap<String, String>,
}

pub async fn find_many(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    args: &FindManyArgs,
    include_total_count: bool,
    nested: &[NestedRelationSelection],
) -> Result<FindManyResult, RunnerError> {
    let model = schema.object(object_name_singular)?;
    let alias = object_name_singular;
    validate_pagination_args(args)?;
    let page_size = page_size(args)?;
    let is_forward = args.before.is_none();
    let pagination_direction = if is_forward {
        CursorPaginationDirection::Forward
    } else {
        CursorPaginationDirection::Backward
    };
    let mut suffixes = Suffixes::new();
    let mut parameter_map = Map::new();
    let root_filter = compile_runner_filter(
        schema,
        object_name_singular,
        args.filter.as_ref(),
        &mut suffixes,
    )?;
    let mut joins = RelationJoins::default();
    collect_filter_relation_joins(
        schema,
        object_name_singular,
        alias,
        args.filter.as_ref(),
        &mut joins,
    )?;
    let mut orders = parse_order_by(
        schema,
        object_name_singular,
        alias,
        args.order_by.as_ref(),
        &mut joins,
    )?;
    orders.push(id_order(model, alias)?);

    let mut clauses;

    if let Some(filter) = root_filter {
        clauses = filter.clauses;
        parameter_map.extend(filter.params);
    } else {
        clauses = Vec::new();
    }
    let total_filter_expression =
        resolve_where_clauses(schema, object_name_singular, alias, &clauses)?;
    let cursor = get_cursor(&crate::query::PaginationArgs {
        after: args.after.clone(),
        before: args.before.clone(),
    })
    .map_err(RunnerError::from_cursor)?;

    if let Some(cursor_data) = cursor {
        let specs = orders
            .iter()
            .map(|order| order.spec.clone())
            .collect::<Vec<_>>();
        let conditions = compute_cursor_arg_filter(&cursor_data, &specs, is_forward)
            .map_err(RunnerError::from_cursor)?;
        let merged = merge_cursor_conditions(&conditions);
        let cursor_filter =
            compile_runner_filter(schema, object_name_singular, Some(&merged), &mut suffixes)?;

        if let Some(filter) = cursor_filter {
            clauses.extend(filter.clauses);
            parameter_map.extend(filter.params);
        }
    }

    let where_expression = resolve_where_clauses(schema, object_name_singular, alias, &clauses)?;
    let deleted_predicate = (!args.with_deleted && model.shape.has_deleted_at_column).then(|| {
        format!(
            "{}.{} IS NULL",
            quote_identifier(alias).expect("root alias is valid"),
            quote_identifier("deletedAt").expect("fixed column is valid")
        )
    });
    let mut predicates = Vec::new();

    if !where_expression.is_empty() {
        predicates.push(where_expression);
    }

    if let Some(predicate) = &deleted_predicate {
        predicates.push(predicate.clone());
    }

    let where_sql = if predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", predicates.join(" AND "))
    };
    let mut total_predicates = Vec::new();

    if !total_filter_expression.is_empty() {
        total_predicates.push(total_filter_expression);
    }

    if let Some(predicate) = &deleted_predicate {
        total_predicates.push(predicate.clone());
    }

    let total_where_sql = if total_predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", total_predicates.join(" AND "))
    };
    let from_sql = from_with_joins(model, alias, &joins)?;
    let selection = Selection::from_fields(args.selected_field_names.clone());
    let mut columns = select_columns(model, &selection)?;

    if selection.field_names.is_empty() || !columns.iter().any(|column| column.column_name == "id")
    {
        if let Some(id) = model.shape.column("id") {
            if !columns.iter().any(|column| column.column_name == "id") {
                columns.insert(0, id.clone());
            }
        }
    }

    let mapped_columns = columns
        .into_iter()
        .map(|column| {
            let result_alias = super::base::column_result_alias(alias, &column.column_name);
            (column, result_alias.clone(), result_alias)
        })
        .collect::<Vec<_>>();
    let scan_values = orders
        .iter()
        .enumerate()
        .map(|(index, order)| (order.expression.clone(), format!("orderBy{index:020}")))
        .collect::<Vec<_>>();
    let select_sql = render_select_list(alias, &mapped_columns, &scan_values)?;
    let order_sql = orders
        .iter()
        .map(|order| {
            format!(
                "{} {} {}",
                order.expression,
                order.order.order.as_str(),
                order.order.nulls.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let fetched_limit = page_size.saturating_add(1);
    let mut sql = format!("SELECT {select_sql} FROM {from_sql}{where_sql}");

    if !order_sql.is_empty() {
        sql.push_str(&format!(" ORDER BY {order_sql}"));
    }

    sql.push_str(&format!(" LIMIT {fetched_limit}"));

    if let Some(offset) = args.offset {
        sql.push_str(&format!(" OFFSET {offset}"));
    }

    let compiled = crate::query::compile::compile_named_parameters(&sql, &parameter_map)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let raw_rows = execute_json_rows(pool, &compiled.text, &compiled.values).await?;
    let page = build_cursor_page(
        raw_rows,
        page_size,
        pagination_direction,
        args.after.is_some(),
        args.before.is_some(),
    );
    let id_alias = super::base::column_result_alias(alias, "id");
    let mut cursor_values_by_id = std::collections::HashMap::new();

    for row in &page.items {
        let Some(id) = row
            .get(&id_alias)
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        let mut values = Vec::with_capacity(orders.len());

        for (index, order) in orders.iter().enumerate() {
            let value = row
                .get(&format!("orderBy{index:020}"))
                .cloned()
                .unwrap_or(Value::Null);
            values.push((&order.spec, value));
        }

        cursor_values_by_id.insert(id.clone(), build_cursor_data(&values, &id));
    }

    let cursor_datas = page
        .items
        .iter()
        .filter_map(|row| {
            let id = row.get(&id_alias).and_then(Value::as_str)?.to_string();
            cursor_values_by_id.get(&id).cloned()
        })
        .collect::<Vec<_>>();
    let page_info = get_page_info(
        &cursor_datas,
        &page.page_info,
        &crate::query::cursor::encode_cursor_data,
    );
    let records = format_result_rows(model, &mapped_columns, &page.items)?;

    let total_count = if include_total_count {
        Some(count_matching_records(pool, &from_sql, &total_where_sql, &parameter_map).await?)
    } else {
        None
    };
    let mut result = FindManyResult {
        records,
        page_info,
        total_count,
        cursors: cursor_values_by_id
            .into_iter()
            .map(|(id, cursor)| (id, crate::query::cursor::encode_cursor_data(&cursor)))
            .collect(),
    };

    if !nested.is_empty() {
        super::nested::load_nested_relations(
            pool,
            schema,
            object_name_singular,
            &mut result.records,
            nested,
        )
        .await?;
    }

    Ok(result)
}

async fn count_matching_records(
    pool: &sqlx::PgPool,
    from_sql: &str,
    where_sql: &str,
    parameters: &Map<String, Value>,
) -> Result<i64, RunnerError> {
    let sql = format!("SELECT count(*)::bigint AS count FROM {from_sql}{where_sql}");
    let compiled = crate::query::compile::compile_named_parameters(&sql, parameters)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let rows = execute_json_rows(pool, &compiled.text, &compiled.values).await?;

    rows.first()
        .and_then(|row| row.get("count").and_then(Value::as_i64))
        .ok_or_else(|| RunnerError::invalid_query("the aggregate query did not return a count"))
}

pub fn validate_pagination_args(args: &FindManyArgs) -> Result<(), RunnerError> {
    let first_present = args.first.is_some_and(|value| value != 0);
    let last_present = args.last.is_some_and(|value| value != 0);

    if first_present && last_present {
        return Err(RunnerError::args_conflict("first", "last"));
    }

    if args.after.is_some() && args.before.is_some() {
        return Err(RunnerError::args_conflict("after", "before"));
    }

    if args.before.is_some() && first_present {
        return Err(RunnerError::args_conflict("before", "first"));
    }

    if args.after.is_some() && last_present {
        return Err(RunnerError::args_conflict("after", "last"));
    }

    if args.first.is_some_and(|value| value == usize::MAX)
        || args.last.is_some_and(|value| value == usize::MAX)
    {
        return Err(RunnerError::invalid_page_argument());
    }

    Ok(())
}

pub fn page_size(args: &FindManyArgs) -> Result<usize, RunnerError> {
    if let Some(first) = args.first {
        if first == usize::MAX {
            return Err(RunnerError::invalid_page_argument());
        }

        return Ok(first.min(QUERY_MAX_RECORDS));
    }

    if let Some(last) = args.last {
        if last == usize::MAX {
            return Err(RunnerError::invalid_page_argument());
        }

        return Ok(last.min(QUERY_MAX_RECORDS));
    }

    Ok(DEFAULT_PAGE_SIZE)
}

#[derive(Default)]
pub struct RelationJoins {
    joins: Vec<(String, String)>,
    aliases: std::collections::HashSet<String>,
}

impl RelationJoins {
    pub fn add(&mut self, alias: String, sql: String) {
        if self.aliases.insert(alias.clone()) {
            self.joins.push((alias, sql));
        }
    }

    pub fn sql(&self) -> String {
        self.joins
            .iter()
            .map(|(_, sql)| sql.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub(crate) fn from_with_joins(
    model: &super::base::ObjectModel,
    alias: &str,
    joins: &RelationJoins,
) -> Result<String, RunnerError> {
    let table = format!(
        "{}.{} AS {}",
        quote_identifier(&model.shape.schema_name)?,
        quote_identifier(&model.shape.table_name)?,
        quote_identifier(alias)?
    );

    if joins.joins.is_empty() {
        return Ok(table);
    }

    Ok(format!("{table} {}", joins.sql()))
}

pub fn add_to_one_join(
    schema: &RunnerSchema,
    object_name: &str,
    alias: &str,
    relation_field_name: &str,
    joins: &mut RelationJoins,
) -> Result<(), RunnerError> {
    let parent = schema.object(object_name)?;
    let (_, relation, child) = schema.relation(object_name, relation_field_name)?;
    let Some(join_column) = relation.join_column_name.as_deref() else {
        return Err(RunnerError::invalid_query(
            "only to-one relations are supported in inline joins",
        ));
    };

    let child_alias = format!("{alias}_{relation_field_name}");
    let sql = format!(
        "LEFT JOIN {}.{} AS {} ON {}.{} = {}.{}",
        quote_identifier(&child.shape.schema_name)?,
        quote_identifier(&child.shape.table_name)?,
        quote_identifier(&child_alias)?,
        quote_identifier(alias)?,
        quote_identifier(join_column)?,
        quote_identifier(&child_alias)?,
        quote_identifier("id")?
    );

    joins.add(child_alias, sql);
    let _ = parent;

    Ok(())
}

pub(crate) fn collect_filter_relation_joins(
    schema: &RunnerSchema,
    object_name: &str,
    alias: &str,
    filter: Option<&Value>,
    joins: &mut RelationJoins,
) -> Result<(), RunnerError> {
    let Some(entries) = filter.and_then(Value::as_object) else {
        return Ok(());
    };

    for (key, value) in entries {
        if matches!(key.as_str(), "and" | "or" | "not") {
            let values = match value {
                Value::Array(items) => items.iter().collect::<Vec<_>>(),
                item => vec![item],
            };

            for item in values {
                collect_filter_relation_joins(schema, object_name, alias, Some(item), joins)?;
            }

            continue;
        }

        if let crate::query::filter::model::FilterKeyInfo::Relation { to_many, .. } = schema
            .lookup(object_name, key)
            .unwrap_or(crate::query::filter::model::FilterKeyInfo::Scalar(
                crate::metadata::flat::FieldMetadataType::Text,
            ))
        {
            if !to_many {
                add_to_one_join(schema, object_name, alias, key, joins)?;
            }
        }
    }

    Ok(())
}

pub fn parse_order_by(
    schema: &RunnerSchema,
    object_name: &str,
    alias: &str,
    order_by: Option<&Value>,
    joins: &mut RelationJoins,
) -> Result<Vec<ResolvedOrder>, RunnerError> {
    let Some(value) = order_by else {
        return Ok(Vec::new());
    };

    let entries = match value {
        Value::Array(items) => items.iter().collect::<Vec<_>>(),
        item => vec![item],
    };
    let mut orders = Vec::new();

    for entry in entries {
        parse_order_entry(
            schema,
            object_name,
            alias,
            entry,
            &mut Vec::new(),
            joins,
            &mut orders,
        )?;
    }

    Ok(orders)
}

#[allow(clippy::too_many_arguments)]
fn parse_order_entry(
    schema: &RunnerSchema,
    object_name: &str,
    alias: &str,
    value: &Value,
    path: &mut Vec<String>,
    joins: &mut RelationJoins,
    orders: &mut Vec<ResolvedOrder>,
) -> Result<(), RunnerError> {
    let Some(entries) = value.as_object() else {
        return Err(RunnerError::invalid_query("orderBy must be an object"));
    };

    for (key, value) in entries {
        if let Some(parsed_direction) = value.as_str() {
            let resolved = scalar_order(
                schema,
                object_name,
                alias,
                key,
                direction(parsed_direction)?,
                path,
            )?;
            orders.push(resolved);
            continue;
        }

        match schema.lookup(object_name, key) {
            Some(crate::query::filter::model::FilterKeyInfo::Relation { to_many, .. }) => {
                if to_many {
                    return Err(RunnerError::invalid_query(
                        "only to-one relations are supported in orderBy",
                    ));
                }

                add_to_one_join(schema, object_name, alias, key, joins)?;
                path.push(key.clone());
                parse_order_entry(
                    schema,
                    &schema.relation(object_name, key)?.2.object.name_singular,
                    &format!("{alias}_{key}"),
                    value,
                    path,
                    joins,
                    orders,
                )?;
                path.pop();
            }
            Some(crate::query::filter::model::FilterKeyInfo::Composite { .. }) => {
                path.push(key.clone());
                parse_order_entry(schema, object_name, alias, value, path, joins, orders)?;
                path.pop();
            }
            Some(_) => {
                return Err(RunnerError::invalid_query(
                    "orderBy leaf must be a direction",
                ))
            }
            None => return Err(RunnerError::unknown_column(key, object_name)),
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn scalar_order(
    schema: &RunnerSchema,
    object_name: &str,
    alias: &str,
    key: &str,
    parsed: OrderByDirection,
    path: &mut [String],
) -> Result<ResolvedOrder, RunnerError> {
    let model = schema.object(object_name)?;
    let mut column_name = key.to_string();
    let mut field_type = FieldMetadataType::Text;
    let mut nullable = true;
    let mut kind = LeafKind::Scalar;

    if let Some(column) = model.shape.column(key) {
        if column.composite_parent_field_name.is_none() {
            column_name = column.column_name.clone();
            field_type = column.field_metadata_type;
            nullable = model
                .fields_by_name
                .get(&column.field_name)
                .map(|field| field.is_nullable)
                .unwrap_or(true);
        }
    }

    if path.len() > 1 {
        kind = LeafKind::Relation;
    }

    if let Some(parent) = path.last().cloned() {
        if let Some(parent_field) = model.fields_by_name.get(&parent) {
            if crate::workspace_migrations::column_defs::composite_types::is_composite_field_metadata_type(
                crate::workspace_migrations::runner::ddl_field_type(parent_field.field_type),
            ) {
                column_name = composite_column(model, &parent, key)?;
                field_type = model
                    .shape
                    .column(&column_name)
                    .map(|column| column.field_metadata_type)
                    .unwrap_or_else(|| {
                        model
                            .fields_by_name
                            .get(key)
                            .map(|field| field.field_type)
                            .unwrap_or(FieldMetadataType::Text)
                    });
                kind = LeafKind::Composite;
                nullable = parent_field.is_nullable;
            }
        }
    }

    if let Some(field) = model.fields_by_name.get(key) {
        if path.is_empty()
            && crate::workspace_migrations::column_defs::composite_types::is_composite_field_metadata_type(
                crate::workspace_migrations::runner::ddl_field_type(field.field_type),
            )
        {
            return Err(RunnerError::unknown_sub_field(key, "", Vec::new()));
        }
    }

    let mut leaf_path = path.to_owned();
    leaf_path.push(key.to_string());
    let expression = order_expression(model, alias, &column_name, field_type)?;
    let order = crate::query::order::convert_order_by_to_find_options_order(parsed.as_str(), true)
        .map_err(RunnerError::from_order)?;

    Ok(ResolvedOrder {
        spec: OrderByLeafSpec {
            path: leaf_path,
            direction: parsed,
            kind,
            can_field_hold_null_value: nullable,
        },
        expression,
        order,
    })
}

fn id_order(model: &super::base::ObjectModel, alias: &str) -> Result<ResolvedOrder, RunnerError> {
    let id_column = model
        .shape
        .column("id")
        .ok_or_else(|| RunnerError::unknown_column("id", &model.object.name_singular))?;
    let order = crate::query::order::convert_order_by_to_find_options_order(
        DEFAULT_ID_ORDER_BY_TIEBREAKER.as_str(),
        true,
    )
    .map_err(RunnerError::from_order)?;

    Ok(ResolvedOrder {
        spec: OrderByLeafSpec {
            path: vec![String::from("id")],
            direction: DEFAULT_ID_ORDER_BY_TIEBREAKER,
            kind: LeafKind::Scalar,
            can_field_hold_null_value: false,
        },
        expression: order_expression(
            model,
            alias,
            &id_column.column_name,
            id_column.field_metadata_type,
        )?,
        order,
    })
}

pub fn unused_relation_type_marker(relation_type: RelationType) -> RelationType {
    relation_type
}

pub fn compiled_filter_clauses(
    filter: Option<CompiledFilter>,
) -> Vec<crate::query::filter::model::WhereClause> {
    filter.map(|filter| filter.clauses).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_args_conflicts_match_the_upstream_truthiness_matrix() {
        let base = FindManyArgs {
            ..Default::default()
        };

        assert!(validate_pagination_args(&base).is_ok());

        let both = FindManyArgs {
            first: Some(1),
            last: Some(1),
            ..Default::default()
        };
        let error = validate_pagination_args(&both).unwrap_err();
        assert_eq!(error.code(), "ARGS_CONFLICT");

        let zero_first_and_last = FindManyArgs {
            first: Some(0),
            last: Some(0),
            ..Default::default()
        };
        assert!(validate_pagination_args(&zero_first_and_last).is_ok());

        let cursors = FindManyArgs {
            after: Some(String::from("a")),
            before: Some(String::from("b")),
            ..Default::default()
        };
        assert_eq!(
            validate_pagination_args(&cursors).unwrap_err().code(),
            "ARGS_CONFLICT"
        );

        let before_first = FindManyArgs {
            first: Some(1),
            before: Some(String::from("b")),
            ..Default::default()
        };
        assert_eq!(
            validate_pagination_args(&before_first).unwrap_err().code(),
            "ARGS_CONFLICT"
        );

        let after_last = FindManyArgs {
            last: Some(1),
            after: Some(String::from("a")),
            ..Default::default()
        };
        assert_eq!(
            validate_pagination_args(&after_last).unwrap_err().code(),
            "ARGS_CONFLICT"
        );
    }

    #[test]
    fn page_size_uses_first_then_last_then_default_and_caps_at_200() {
        let args = FindManyArgs {
            ..Default::default()
        };
        assert_eq!(page_size(&args).unwrap(), 200);

        let args = FindManyArgs {
            first: Some(201),
            ..Default::default()
        };
        assert_eq!(page_size(&args).unwrap(), 200);

        let args = FindManyArgs {
            first: Some(0),
            last: Some(7),
            ..Default::default()
        };
        assert_eq!(page_size(&args).unwrap(), 0);

        let args = FindManyArgs {
            first: Some(usize::MAX),
            ..Default::default()
        };
        assert_eq!(
            validate_pagination_args(&args).unwrap_err().code(),
            "INVALID_ARGS"
        );
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::metadata::flat::load_workspace_flat_maps;
    use crate::query::compile::compile_named_parameters;
    use crate::query::repository::build_insert_rows;
    use crate::query::repository::build_insert_statement;
    use crate::query::runner::find_one;
    use crate::query::runner::find_one::FindOneArgs;
    use crate::state::AppState;
    use nudge_core::NodeEnv;
    use serde_json::json;
    use uuid::Uuid;

    static DB_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn test_config(pg_database_url: String) -> nudge_core::Config {
        nudge_core::Config {
            node_env: NodeEnv::Test,
            node_port: 3997,
            server_url: "http://localhost:3997".to_string(),
            pg_database_url,
            pg_pool_max_connections: 4,
            redis_url: "redis://127.0.0.1:6398".to_string(),
            redis_cache_prefix: String::new(),
            app_secret: "runner-db-test-secret".to_string(),
            encryption_key: None,
            fallback_encryption_key: None,
            access_token_expires_in_secs: 1800,
            refresh_token_expires_in_secs: 3600,
            login_token_expires_in_secs: 900,
            workspace_agnostic_token_expires_in_secs: 1800,
            refresh_token_reuse_grace_period_secs: 60,
            frontend_url: "http://localhost:3997".to_string(),
            auth_cookie_same_site: "lax".to_string(),
            auth_cookie_allowed_origins: String::new(),
            session_absolute_lifetime_secs: 3600,
            session_idle_timeout_secs: 3600,
            is_cloudflare_geolocation_enabled: false,
            workspace_schema_ddl_locked: false,
        }
    }

    struct Seeded {
        state: AppState,
        workspace_id: Uuid,
        schema_name: String,
    }

    async fn seeded_workspace(url: &str) -> Option<Seeded> {
        let state = AppState::connect(&test_config(url.to_string()))
            .await
            .expect("runner db test state");

        crate::workspace_migrations::ddl::test_support::migrate_core(url)
            .await
            .expect("runner core migrations");

        let workspace_id = Uuid::new_v4();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let application_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let mut fixture = state.core_pool.begin().await.expect("runner fixture");

        sqlx::query(
            "INSERT INTO core.\"workspace\" \
             (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
              \"workspaceCustomApplicationId\", \"isPasswordAuthEnabled\", \"databaseSchema\") \
             VALUES ($1, 'Runner Parity', $2, 'PENDING_CREATION', $3, $4, true, $5)",
        )
        .bind(workspace_id)
        .bind(Uuid::new_v4())
        .bind(format!(
            "runner-{}",
            &workspace_id.simple().to_string()[..8]
        ))
        .bind(application_id)
        .bind(&schema_name)
        .execute(&mut *fixture)
        .await
        .expect("insert runner workspace");

        sqlx::query(
            "INSERT INTO core.\"application\" \
             (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", \"workspaceId\") \
             VALUES ($1, $2, 'Runner Application', 'LOCAL', 'INSTALLED', '', $3)",
        )
        .bind(application_id)
        .bind(Uuid::new_v4())
        .bind(workspace_id)
        .execute(&mut *fixture)
        .await
        .expect("insert runner application");

        sqlx::query(
            "INSERT INTO core.\"role\" \
             (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\") \
             VALUES ($1, $2, 'Member', $3, $4)",
        )
        .bind(role_id)
        .bind(Uuid::new_v4())
        .bind(application_id)
        .bind(workspace_id)
        .execute(&mut *fixture)
        .await
        .expect("insert runner role");

        sqlx::query(
            "UPDATE core.\"workspace\" SET \"activationStatus\" = 'CREATED', \
                \"defaultRoleId\" = $2 WHERE id = $1",
        )
        .bind(workspace_id)
        .bind(role_id)
        .execute(&mut *fixture)
        .await
        .expect("activate runner workspace");

        fixture.commit().await.expect("commit runner fixture");

        sqlx::query(&format!("CREATE SCHEMA \"{schema_name}\""))
            .execute(&state.core_pool)
            .await
            .expect("create runner schema");

        let outcome = crate::standard_objects::seed_standard_objects(&state, workspace_id)
            .await
            .expect("seed runner standard objects");
        assert!(outcome.seeded);

        Some(Seeded {
            state,
            workspace_id,
            schema_name,
        })
    }

    fn record(json: &str) -> serde_json::Map<String, Value> {
        serde_json::from_str::<Value>(json)
            .expect("record json")
            .as_object()
            .expect("record object")
            .clone()
    }

    async fn insert_rows(
        state: &AppState,
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
                    .and_then(|value| uuid::Uuid::parse_str(value).ok())
                    .expect("uuid insert value");
                query = query.bind(value);
            } else {
                query = query.bind(value.as_str().expect("text insert value").to_string());
            }
        }

        query
            .fetch_all(&state.core_pool)
            .await
            .expect("insert executes")
    }

    #[tokio::test]
    async fn find_many_filters_orders_paginates_and_loads_to_one_relations() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };

        let _guard = DB_TEST_LOCK.lock().await;
        let seeded = seeded_workspace(&url)
            .await
            .expect("seeded runner workspace");
        let state = &seeded.state;
        let maps = load_workspace_flat_maps(&state.core_pool, seeded.workspace_id)
            .await
            .expect("load runner flat maps");
        let objects = maps
            .object_metadata_maps
            .by_universal_identifier
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let schema = RunnerSchema::build(
            &seeded.workspace_id,
            &objects,
            &maps.field_metadata_maps,
            &maps.index_metadata_maps,
        )
        .expect("build runner schema");

        let company_shape = schema.object("company").expect("company").shape.clone();
        let person_shape = schema.object("person").expect("person").shape.clone();

        let company_rows = build_insert_rows(&company_shape, &[record(r#"{"name": "Nudge Inc"}"#)])
            .expect("company insert rows");
        let company_sql = build_insert_statement(
            &company_shape,
            &company_rows.column_names,
            &company_rows.rows,
            &["id".to_string()],
            false,
        )
        .expect("company insert statement");
        let company_ids = insert_rows(
            state,
            &company_shape,
            &company_sql,
            &company_rows.parameters,
            &company_rows.column_names,
        )
        .await;
        assert_eq!(company_ids.len(), 1);
        let company_id = company_ids[0];

        let person_rows = build_insert_rows(
            &person_shape,
            &[
                record(
                    r#"{"name": {"firstName": "Ada", "lastName": "Lovelace"}, "jobTitle": "Engineer", "companyId": "COMPANY_ID"}"#,
                ),
                record(
                    r#"{"name": {"firstName": "Grace", "lastName": "Hopper"}, "jobTitle": "Admiral", "companyId": "COMPANY_ID"}"#,
                ),
                record(r#"{"name": {"firstName": "Alan", "lastName": "Turing"}, "jobTitle": "Mathematician"}"#),
            ]
            .into_iter()
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
        .expect("person insert rows");
        let person_sql = build_insert_statement(
            &person_shape,
            &person_rows.column_names,
            &person_rows.rows,
            &["id".to_string()],
            false,
        )
        .expect("person insert statement");
        let person_ids = insert_rows(
            state,
            &person_shape,
            &person_sql,
            &person_rows.parameters,
            &person_rows.column_names,
        )
        .await;
        assert_eq!(person_ids.len(), 3);

        let simple = find_many(
            &state.core_pool,
            &schema,
            "person",
            &FindManyArgs {
                filter: Some(json!({"name": {"firstName": {"eq": "Ada"}}})),
                order_by: Some(json!([{"jobTitle": "AscNullsLast"}])),
                first: Some(1),
                selected_field_names: vec![
                    String::from("id"),
                    String::from("name"),
                    String::from("jobTitle"),
                ],
                ..Default::default()
            },
            true,
            &[],
        )
        .await
        .expect("simple find many");

        assert_eq!(simple.records.len(), 1);
        assert_eq!(simple.total_count, Some(1));
        assert_eq!(simple.records[0]["name"]["firstName"], json!("Ada"));
        assert_eq!(simple.records[0]["jobTitle"], json!("Engineer"));
        assert!(!simple.page_info.has_next_page);

        let relation_filter = find_many(
            &state.core_pool,
            &schema,
            "person",
            &FindManyArgs {
                filter: Some(json!({"company": {"name": {"eq": "Nudge Inc"}}})),
                order_by: Some(json!([{"jobTitle": "AscNullsLast"}])),
                first: Some(2),
                selected_field_names: vec![
                    String::from("id"),
                    String::from("name"),
                    String::from("company"),
                ],
                ..Default::default()
            },
            true,
            &[],
        )
        .await
        .expect("relation filtered find many");

        assert_eq!(relation_filter.total_count, Some(2));
        assert_eq!(relation_filter.records.len(), 2);
        assert_eq!(
            relation_filter.records[0]["name"]["firstName"],
            json!("Grace")
        );
        assert_eq!(
            relation_filter.records[0]["company"],
            Value::String(company_id.to_string())
        );

        let nested = find_many(
            &state.core_pool,
            &schema,
            "person",
            &FindManyArgs {
                filter: Some(json!({"name": {"firstName": {"eq": "Ada"}}})),
                selected_field_names: vec![String::from("id"), String::from("company")],
                ..Default::default()
            },
            false,
            &[NestedRelationSelection {
                relation_field_name: String::from("company"),
                selected_field_names: vec![String::from("id"), String::from("name")],
                first: None,
            }],
        )
        .await
        .expect("nested relation find many");

        assert_eq!(nested.records[0]["company"]["name"], json!("Nudge Inc"));

        let one = find_one(
            &state.core_pool,
            &schema,
            "person",
            &FindOneArgs {
                filter: Some(json!({"id": {"eq": person_ids[0].to_string()}})),
                selected_field_names: vec![String::from("id"), String::from("jobTitle")],
                ..Default::default()
            },
            &[],
        )
        .await
        .expect("find one")
        .expect("record exists");
        assert_eq!(one["jobTitle"], json!("Engineer"));

        let missing = find_one(
            &state.core_pool,
            &schema,
            "person",
            &FindOneArgs {
                filter: Some(json!({"id": {"eq": Uuid::nil().to_string()}})),
                ..Default::default()
            },
            &[],
        )
        .await
        .expect("missing find one does not error");
        assert!(missing.is_none());

        let conflict = find_many(
            &state.core_pool,
            &schema,
            "person",
            &FindManyArgs {
                first: Some(1),
                last: Some(1),
                ..Default::default()
            },
            false,
            &[],
        )
        .await
        .unwrap_err();
        assert_eq!(conflict.code(), "ARGS_CONFLICT");

        sqlx::query(&format!("DROP SCHEMA \"{}\" CASCADE", seeded.schema_name))
            .execute(&state.core_pool)
            .await
            .expect("drop runner schema");
    }
}
