use std::sync::Arc;
use std::sync::OnceLock;

use base64::Engine;
use regex::Regex;
use serde_json::Map;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;

use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FlatObjectMetadata;
use crate::metadata::names::compute_composite_column_name;
use crate::permissions::assert_operation_allowed;
use crate::permissions::ObjectRecordsOperationType;
use crate::query::cursor::decode_cursor;
use crate::query::cursor::encode_cursor_data;
use crate::query::filter::format_search_terms;
use crate::query::filter::Suffixes;
use crate::query::repository::quote_identifier;

use super::base::bind_json_value;
use super::base::compile_runner_filter;
use super::base::execute_json_rows;
use super::base::resolve_where_clauses;
use super::base::row_to_json;
use super::base::RunnerSchema;
use super::permissions::validate_filter_read;
use super::permissions::validate_read;
use super::rls::row_access_filter;
use super::RunnerError;

const OBJECT_METADATA_ITEMS_CHUNK_SIZE: usize = 5;
const SEARCH_VECTOR_FIELD_NAME: &str = "searchVector";
const SEARCH_OBJECT_LABEL_CATALOG: &str = include_str!("search_object_labels.json");
const CHANNEL_VISIBILITY_OBJECTS: [&str; 10] = [
    "blocklist",
    "message",
    "messageThread",
    "messageParticipant",
    "messageChannelMessageAssociation",
    "messageChannelMessageAssociationMessageFolder",
    "messageThreadSubscriber",
    "calendarEvent",
    "calendarChannelEventAssociation",
    "calendarEventParticipant",
];

#[derive(Debug, Clone)]
pub struct SearchArgs {
    pub search_input: String,
    pub limit: usize,
    pub after: Option<String>,
    pub included_object_name_singulars: Vec<String>,
    pub excluded_object_name_singulars: Vec<String>,
    pub filter: Option<Value>,
    pub locale: String,
}

#[derive(Debug, Clone)]
pub struct SearchRecord {
    pub record_id: String,
    pub object_name_singular: String,
    pub object_label_singular: String,
    pub label: String,
    pub image_url: String,
    pub ts_rank_cd: f64,
    pub ts_rank: f64,
}

#[derive(Debug, Clone)]
struct SearchCursor {
    ts_rank_cd: f64,
    ts_rank: f64,
    last_record_ids_per_object: Map<String, Value>,
}

fn can_read_object(
    permissions: Option<&crate::permissions::RoleRecordsPermissions>,
    object: &FlatObjectMetadata,
) -> bool {
    permissions.is_none_or(|permissions| {
        assert_operation_allowed(permissions, object.id, ObjectRecordsOperationType::Read).is_ok()
    })
}

fn effective_is_active(object: &FlatObjectMetadata) -> bool {
    override_property(object, "isActive")
        .and_then(Value::as_bool)
        .unwrap_or(object.is_active)
}

fn effective_label_singular(object: &FlatObjectMetadata, locale: &str) -> String {
    if let Some(translation) =
        override_property_at_path(object, &["translations", locale, "labelSingular"])
            .and_then(Value::as_str)
    {
        return translation.to_string();
    }
    override_property(object, "labelSingular")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| standard_label_translation(&object.label_singular, locale).map(str::to_string))
        .unwrap_or_else(|| object.label_singular.clone())
}

fn standard_label_translation(label: &str, locale: &str) -> Option<&'static str> {
    static CATALOG: OnceLock<Value> = OnceLock::new();
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(SEARCH_OBJECT_LABEL_CATALOG).expect("valid label catalog")
    });
    let message_id = object_label_message_id(label);
    catalog.get(locale)?.get(message_id.as_str())?.as_str()
}

fn object_label_message_id(label: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(label.as_bytes());
    hash.update([0x1f]);
    hash.update(b"objectMetadata.labelSingular");
    base64::engine::general_purpose::STANDARD.encode(hash.finalize())[..6].to_string()
}

fn override_property_at_path<'a>(
    object: &'a FlatObjectMetadata,
    path: &[&str],
) -> Option<&'a Value> {
    let overrides_value = object.overrides.as_ref()?;
    let overrides = overrides_value.as_object()?;
    let read_path = |value: &'a Value| {
        let mut current = Some(value);
        for key in path {
            current = current.and_then(|entry| entry.get(*key));
        }
        current
    };
    if let Some(value) = read_path(overrides_value) {
        return Some(value);
    }
    let owner = object.application_universal_identifier.as_str();
    overrides
        .iter()
        .filter(|(author, _)| author.as_str() != owner)
        .find_map(|(_, entry)| read_path(entry))
        .or_else(|| overrides.get(owner).and_then(read_path))
}

fn override_property<'a>(object: &'a FlatObjectMetadata, property: &str) -> Option<&'a Value> {
    override_property_at_path(object, &[property])
}

fn select_objects(objects: &[FlatObjectMetadata], args: &SearchArgs) -> Vec<FlatObjectMetadata> {
    objects
        .iter()
        .filter(|object| effective_is_active(object))
        .filter(|object| {
            if !args.included_object_name_singulars.is_empty() {
                return !CHANNEL_VISIBILITY_OBJECTS.contains(&object.name_singular.as_str())
                    && args
                        .included_object_name_singulars
                        .contains(&object.name_singular)
                    && !args
                        .excluded_object_name_singulars
                        .contains(&object.name_singular);
            }
            object.is_searchable
                && !args
                    .excluded_object_name_singulars
                    .contains(&object.name_singular)
        })
        .cloned()
        .collect()
}

pub async fn search(
    pool: &sqlx::PgPool,
    schema: Arc<RunnerSchema>,
    objects: &[FlatObjectMetadata],
    args: &SearchArgs,
    ilike_fallback_timeout_ms: u64,
    allow_requests_to_twenty_icons: bool,
) -> Result<Value, RunnerError> {
    if args.limit > 100 {
        return Err(RunnerError::new(
            "BAD_REQUEST",
            "Limit cannot exceed 100 items",
        ));
    }
    let cursor = args
        .after
        .as_deref()
        .map(decode_search_cursor)
        .transpose()?;
    let mut selected_objects = select_objects(objects, args);
    if let Some(permissions) = schema.records_permissions() {
        selected_objects.retain(|object| can_read_object(Some(permissions), object));
    }
    let mut records: Vec<SearchRecord> = Vec::new();
    let mut tasks = tokio::task::JoinSet::new();

    for chunk in selected_objects.chunks(OBJECT_METADATA_ITEMS_CHUNK_SIZE) {
        for (index, object) in chunk.iter().enumerate() {
            let pool = pool.clone();
            let schema = schema.clone();
            let args = args.clone();
            let cursor = cursor.clone();
            let object = object.clone();
            tasks.spawn(async move {
                (
                    index,
                    search_object(
                        &pool,
                        schema,
                        &object,
                        &args,
                        cursor.as_ref(),
                        ilike_fallback_timeout_ms,
                        allow_requests_to_twenty_icons,
                    )
                    .await,
                )
            });
        }
        let mut ordered = vec![Vec::new(); chunk.len()];
        while let Some(result) = tasks.join_next().await {
            let (index, result) = result.map_err(|error| {
                RunnerError::new("RECORD_PERMISSION_QUERY_INVALID", error.to_string())
            })?;
            let batch = match result {
                Ok(batch) => batch,
                Err(error) if error.code() == "PERMISSION_DENIED" => continue,
                Err(error) => return Err(error),
            };
            ordered[index] = batch;
        }
        for batch in ordered {
            records.extend(batch);
        }
    }

    records.sort_by(|left, right| {
        right
            .ts_rank_cd
            .total_cmp(&left.ts_rank_cd)
            .then_with(|| right.ts_rank.total_cmp(&left.ts_rank))
            .then_with(|| {
                object_priority(&right.object_name_singular)
                    .cmp(&object_priority(&left.object_name_singular))
            })
    });
    let has_next_page = records.len() > args.limit;
    records.truncate(args.limit);
    let mut last_record_ids_per_object = cursor
        .as_ref()
        .map(|cursor| cursor.last_record_ids_per_object.clone())
        .unwrap_or_default();
    let mut edges = Vec::with_capacity(records.len());

    for record in records {
        last_record_ids_per_object.insert(
            record.object_name_singular.clone(),
            Value::String(record.record_id.clone()),
        );
        let cursor = encode_cursor_data(&Map::from_iter([
            (
                String::from("lastRanks"),
                serde_json::json!({
                    "tsRankCD": record.ts_rank_cd,
                    "tsRank": record.ts_rank,
                }),
            ),
            (
                String::from("lastRecordIdsPerObject"),
                Value::Object(last_record_ids_per_object.clone()),
            ),
        ]));
        edges.push(serde_json::json!({
            "node": {
                "recordId": record.record_id,
                "objectNameSingular": record.object_name_singular,
                "objectLabelSingular": record.object_label_singular,
                "label": record.label,
                "imageUrl": record.image_url,
                "tsRankCD": record.ts_rank_cd,
                "tsRank": record.ts_rank,
            },
            "cursor": cursor,
        }));
    }

    let end_cursor = edges
        .last()
        .and_then(|edge| edge.get("cursor"))
        .cloned()
        .unwrap_or(Value::Null);
    Ok(serde_json::json!({
        "edges": edges,
        "pageInfo": {
            "endCursor": end_cursor,
            "hasNextPage": has_next_page,
        },
    }))
}

async fn search_object(
    pool: &sqlx::PgPool,
    schema: Arc<RunnerSchema>,
    object: &FlatObjectMetadata,
    args: &SearchArgs,
    cursor: Option<&SearchCursor>,
    ilike_fallback_timeout_ms: u64,
    allow_requests_to_twenty_icons: bool,
) -> Result<Vec<SearchRecord>, RunnerError> {
    let model = schema.object(&object.name_singular)?;
    validate_filter_read(&schema, model, args.filter.as_ref())?;
    let label_columns = label_identifier_columns(object, &schema)?;
    let image_identifier = image_identifier_column(object, &schema);
    let mut selected_columns = label_columns.clone();
    if let Some((column, _)) = &image_identifier {
        selected_columns.push(column.clone());
    }
    selected_columns.dedup();
    let selected_shapes = selected_columns
        .iter()
        .filter_map(|name| model.shape.column(name).cloned())
        .collect::<Vec<_>>();
    validate_read(&schema, model, &selected_shapes)?;
    let mut records = search_vector_records(
        pool,
        &schema,
        object,
        args,
        cursor,
        allow_requests_to_twenty_icons,
    )
    .await?;

    if records.is_empty() && cursor.is_none() && has_cjk_characters(&args.search_input) {
        records = ilike_fallback_records(
            pool,
            &schema,
            object,
            args,
            ilike_fallback_timeout_ms,
            allow_requests_to_twenty_icons,
        )
        .await?;
    }

    Ok(records
        .into_iter()
        .map(|mut record| {
            record.object_label_singular = effective_label_singular(object, &args.locale);
            record
        })
        .collect())
}

async fn search_vector_records(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object: &FlatObjectMetadata,
    args: &SearchArgs,
    cursor: Option<&SearchCursor>,
    allow_requests_to_twenty_icons: bool,
) -> Result<Vec<SearchRecord>, RunnerError> {
    let model = schema.object(&object.name_singular)?;
    let alias = object.name_singular.as_str();
    let mut suffixes = Suffixes::new();
    let mut clauses = Vec::new();
    let mut parameters = Map::new();
    add_common_clauses(
        pool,
        schema,
        object,
        args,
        &mut suffixes,
        &mut clauses,
        &mut parameters,
    )
    .await?;
    let search_terms = format_search_terms(&args.search_input, "and");
    let search_terms_or = format_search_terms(&args.search_input, "or");
    let vector = quote_identifier(SEARCH_VECTOR_FIELD_NAME)?;
    let aliased_vector = format!("{}.{}", quote_identifier(alias)?, vector);
    if search_terms.is_empty() {
        clauses.push(crate::query::filter::WhereClause::sql(
            crate::query::filter::JoinOp::And,
            format!("{aliased_vector} IS NOT NULL"),
        ));
    } else {
        clauses.push(crate::query::filter::WhereClause::sql(
            crate::query::filter::JoinOp::And,
            format!(
                "({aliased_vector} @@ to_tsquery('simple', public.unaccent_immutable(:searchTerms)) OR {aliased_vector} @@ to_tsquery('simple', public.unaccent_immutable(:searchTermsOr)))"
            ),
        ));
    }
    if let Some(cursor) = cursor {
        add_cursor_clause(
            alias,
            &object.name_singular,
            cursor,
            &mut clauses,
            &mut parameters,
        )?;
    }
    parameters.insert(String::from("searchTerms"), Value::String(search_terms));
    parameters.insert(
        String::from("searchTermsOr"),
        Value::String(search_terms_or),
    );
    let select_list = search_select_list(model, schema, alias, true)?;
    let where_sql = resolve_where_clauses(schema, &object.name_singular, alias, &clauses)?;
    let sql = format!(
        "SELECT {select_list} FROM {}.{} AS {} WHERE {where_sql} ORDER BY \"tsRankCD\" DESC, \"tsRank\" DESC, {}.\"id\" ASC NULLS FIRST LIMIT {}",
        quote_identifier(&model.shape.schema_name)?,
        quote_identifier(&model.shape.table_name)?,
        quote_identifier(alias)?,
        quote_identifier(alias)?,
        args.limit + 1
    );
    let compiled = crate::query::compile::compile_named_parameters(&sql, &parameters)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let rows = execute_json_rows(pool, &compiled.text, &compiled.values).await?;
    rows.into_iter()
        .map(|row| {
            search_record_from_row(object, schema, row, true, allow_requests_to_twenty_icons)
        })
        .collect()
}

async fn ilike_fallback_records(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object: &FlatObjectMetadata,
    args: &SearchArgs,
    timeout_ms: u64,
    allow_requests_to_twenty_icons: bool,
) -> Result<Vec<SearchRecord>, RunnerError> {
    let model = schema.object(&object.name_singular)?;
    let alias = object.name_singular.as_str();
    let mut suffixes = Suffixes::new();
    let mut clauses = Vec::new();
    let mut parameters = Map::new();
    add_common_clauses(
        pool,
        schema,
        object,
        args,
        &mut suffixes,
        &mut clauses,
        &mut parameters,
    )
    .await?;
    let vector = format!(
        "public.unaccent_immutable({}.{}::text)",
        quote_identifier(alias)?,
        quote_identifier(SEARCH_VECTOR_FIELD_NAME)?
    );
    for (index, word) in args
        .search_input
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .enumerate()
    {
        let parameter = format!("ilikeFallback{index}");
        clauses.push(crate::query::filter::WhereClause::sql(
            crate::query::filter::JoinOp::And,
            format!("{vector} ILIKE public.unaccent_immutable(:{parameter})"),
        ));
        parameters.insert(
            parameter,
            Value::String(format!("%{}%", escape_for_ilike(word))),
        );
    }
    let select_list = search_select_list(model, schema, alias, false)?;
    let where_sql = resolve_where_clauses(schema, &object.name_singular, alias, &clauses)?;
    let sql = format!(
        "SELECT {select_list} FROM {}.{} AS {} WHERE {where_sql} ORDER BY {}.\"id\" ASC LIMIT {}",
        quote_identifier(&model.shape.schema_name)?,
        quote_identifier(&model.shape.table_name)?,
        quote_identifier(alias)?,
        quote_identifier(alias)?,
        args.limit
    );
    let compiled = crate::query::compile::compile_named_parameters(&sql, &parameters)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let mut transaction = pool.begin().await.map_err(RunnerError::database)?;
    sqlx::query("SELECT set_config('statement_timeout', $1, true)")
        .bind(timeout_ms.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(RunnerError::database)?;
    let mut query = sqlx::query(&compiled.text);
    for value in &compiled.values {
        query = bind_json_value(query, value)?;
    }
    let rows = match query.fetch_all(&mut *transaction).await {
        Ok(rows) => rows,
        Err(error) if is_query_canceled(&error) => return Ok(Vec::new()),
        Err(error) => return Err(RunnerError::database(error)),
    };
    transaction.commit().await.map_err(RunnerError::database)?;
    rows.iter()
        .map(row_to_json)
        .collect::<Result<Vec<_>, RunnerError>>()?
        .into_iter()
        .map(|row| {
            search_record_from_row(object, schema, row, false, allow_requests_to_twenty_icons)
        })
        .collect()
}

async fn add_common_clauses(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object: &FlatObjectMetadata,
    args: &SearchArgs,
    suffixes: &mut Suffixes,
    clauses: &mut Vec<crate::query::filter::WhereClause>,
    parameters: &mut Map<String, Value>,
) -> Result<(), RunnerError> {
    if let Some(filter) = compile_runner_filter(
        schema,
        &object.name_singular,
        args.filter.as_ref(),
        suffixes,
    )? {
        clauses.extend(filter.clauses);
        parameters.extend(filter.params);
    }
    if let Some(access) = row_access_filter(
        pool,
        schema,
        &object.name_singular,
        ObjectRecordsOperationType::Read,
        suffixes,
    )
    .await?
    {
        clauses.extend(access.clauses);
        parameters.extend(access.params);
    }
    let model = schema.object(&object.name_singular)?;
    if model.shape.has_deleted_at_column {
        clauses.push(crate::query::filter::WhereClause::sql(
            crate::query::filter::JoinOp::And,
            format!(
                "{}.{} IS NULL",
                quote_identifier(&object.name_singular)?,
                quote_identifier("deletedAt")?
            ),
        ));
    }
    Ok(())
}

fn add_cursor_clause(
    alias: &str,
    object_name: &str,
    cursor: &SearchCursor,
    clauses: &mut Vec<crate::query::filter::WhereClause>,
    parameters: &mut Map<String, Value>,
) -> Result<(), RunnerError> {
    let ts_rank_cd = format!(
        "ts_rank_cd({}.\"{}\", to_tsquery('simple', public.unaccent_immutable(:searchTerms)))",
        quote_identifier(alias)?,
        SEARCH_VECTOR_FIELD_NAME
    );
    let ts_rank = format!(
        "ts_rank({}.\"{}\", to_tsquery('simple', public.unaccent_immutable(:searchTermsOr)))",
        quote_identifier(alias)?,
        SEARCH_VECTOR_FIELD_NAME
    );
    parameters.insert(String::from("tsRankCDLt"), Value::from(cursor.ts_rank_cd));
    parameters.insert(String::from("tsRankCDEq"), Value::from(cursor.ts_rank_cd));
    parameters.insert(String::from("tsRankLt"), Value::from(cursor.ts_rank));
    parameters.insert(String::from("tsRankEq"), Value::from(cursor.ts_rank));
    let mut condition = format!(
        "({ts_rank_cd} < CAST(:tsRankCDLt AS real) OR ({ts_rank_cd} = CAST(:tsRankCDEq AS real) AND {ts_rank} < CAST(:tsRankLt AS real)) OR ({ts_rank_cd} = CAST(:tsRankCDEq AS real) AND {ts_rank} = CAST(:tsRankEq AS real)"
    );
    if let Some(last_record_id) = cursor
        .last_record_ids_per_object
        .get(object_name)
        .and_then(Value::as_str)
    {
        parameters.insert(
            String::from("lastRecordId"),
            Value::String(last_record_id.to_string()),
        );
        condition.push_str(&format!(
            " AND {}.\"id\" > CAST(:lastRecordId AS uuid)",
            quote_identifier(alias)?
        ));
    }
    condition.push_str("))");
    clauses.push(crate::query::filter::WhereClause::sql(
        crate::query::filter::JoinOp::And,
        condition,
    ));
    Ok(())
}

fn search_select_list(
    model: &super::base::ObjectModel,
    schema: &RunnerSchema,
    alias: &str,
    with_rank: bool,
) -> Result<String, RunnerError> {
    let object = &model.object;
    let label_columns = label_identifier_columns(object, schema)?;
    let mut columns = vec![String::from("id")];
    columns.extend(label_columns);
    if let Some((column, _)) = image_identifier_column(object, schema) {
        columns.push(column);
    }
    columns.dedup();
    let mut selections = columns
        .iter()
        .map(|column| {
            Ok(format!(
                "{}.{} AS {}",
                quote_identifier(alias)?,
                quote_identifier(column)?,
                quote_identifier(column)?
            ))
        })
        .collect::<Result<Vec<_>, RunnerError>>()?;
    if with_rank {
        selections.push(format!(
            "ts_rank_cd({}.\"{}\", to_tsquery('simple', public.unaccent_immutable(:searchTerms))) AS \"tsRankCD\"",
            quote_identifier(alias)?,
            SEARCH_VECTOR_FIELD_NAME
        ));
        selections.push(format!(
            "ts_rank({}.\"{}\", to_tsquery('simple', public.unaccent_immutable(:searchTermsOr))) AS \"tsRank\"",
            quote_identifier(alias)?,
            SEARCH_VECTOR_FIELD_NAME
        ));
    } else {
        selections.push(String::from("0.0 AS \"tsRankCD\""));
        selections.push(String::from("0.0 AS \"tsRank\""));
    }
    Ok(selections.join(", "))
}

fn label_identifier_columns(
    object: &FlatObjectMetadata,
    schema: &RunnerSchema,
) -> Result<Vec<String>, RunnerError> {
    let field_id = object.label_identifier_field_metadata_id.ok_or_else(|| {
        RunnerError::new(
            "LABEL_IDENTIFIER_FIELD_NOT_FOUND",
            "Label identifier field not found",
        )
    })?;
    let field = schema.field_metadata(field_id).ok_or_else(|| {
        RunnerError::new(
            "LABEL_IDENTIFIER_FIELD_NOT_FOUND",
            "Label identifier field not found",
        )
    })?;
    if field.field_type == FieldMetadataType::FullName {
        return Ok(vec![
            compute_composite_column_name(&field.name, "firstName"),
            compute_composite_column_name(&field.name, "lastName"),
        ]);
    }
    Ok(vec![field.name.clone()])
}

fn image_identifier_column(
    object: &FlatObjectMetadata,
    schema: &RunnerSchema,
) -> Option<(String, FieldMetadataType)> {
    if object.name_singular == "workspaceMember" {
        return Some((String::from("avatarUrl"), FieldMetadataType::Text));
    }
    let field_id = object.image_identifier_field_metadata_id?;
    let field = schema.field_metadata(field_id)?;
    match field.field_type {
        FieldMetadataType::Links => Some((
            compute_composite_column_name(&field.name, "primaryLinkUrl"),
            field.field_type,
        )),
        FieldMetadataType::Files => Some((field.name.clone(), field.field_type)),
        _ => None,
    }
}

fn search_record_from_row(
    object: &FlatObjectMetadata,
    schema: &RunnerSchema,
    row: Map<String, Value>,
    with_rank: bool,
    allow_requests_to_twenty_icons: bool,
) -> Result<SearchRecord, RunnerError> {
    let record_id = row
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunnerError::new(
                "RECORD_PERMISSION_QUERY_INVALID",
                "Search record id missing",
            )
        })?
        .to_string();
    let image_identifier = image_identifier_column(object, schema);
    let image_column = image_identifier.as_ref().map(|(column, _)| column.as_str());
    let label = row
        .iter()
        .filter(|(name, _)| {
            name.as_str() != "id"
                && name.as_str() != "tsRankCD"
                && name.as_str() != "tsRank"
                && image_column != Some(name.as_str())
        })
        .map(|(_, value)| label_value(value))
        .collect::<Vec<_>>()
        .join(" ");
    let image_url = match image_identifier {
        Some((column, FieldMetadataType::Links)) if allow_requests_to_twenty_icons => row
            .get(&column)
            .and_then(Value::as_str)
            .and_then(link_favicon_url)
            .unwrap_or_default(),
        Some((column, FieldMetadataType::Text)) => row
            .get(&column)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    };
    Ok(SearchRecord {
        record_id,
        object_name_singular: object.name_singular.clone(),
        object_label_singular: object.label_singular.clone(),
        label,
        image_url,
        ts_rank_cd: if with_rank {
            normalize_rank(row.get("tsRankCD").and_then(Value::as_f64).unwrap_or(0.0))
        } else {
            0.0
        },
        ts_rank: if with_rank {
            normalize_rank(row.get("tsRank").and_then(Value::as_f64).unwrap_or(0.0))
        } else {
            0.0
        },
    })
}

fn normalize_rank(value: f64) -> f64 {
    (value as f32).to_string().parse().unwrap_or(value)
}

fn is_query_canceled(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .is_some_and(|code| code == "57014")
}

fn label_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        _ => value.to_string(),
    }
}

fn link_favicon_url(link: &str) -> Option<String> {
    let trimmed = link.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed.contains("://") || reqwest::Url::parse(trimmed).is_ok() {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let url = reqwest::Url::parse(&normalized).ok()?;
    let hostname = url.host_str()?;
    let domain = hostname.strip_prefix("www.").unwrap_or(hostname);
    Some(format!("https://twenty-icons.com/{domain}"))
}

fn decode_search_cursor(raw: &str) -> Result<SearchCursor, RunnerError> {
    let cursor = decode_cursor(raw).map_err(RunnerError::from_cursor)?;
    let ranks = cursor
        .get("lastRanks")
        .and_then(Value::as_object)
        .ok_or_else(|| RunnerError::new("INVALID_CURSOR", "Invalid cursor: missing lastRanks"))?;
    let ts_rank_cd = ranks
        .get("tsRankCD")
        .and_then(Value::as_f64)
        .ok_or_else(|| RunnerError::new("INVALID_CURSOR", "Invalid cursor: missing tsRankCD"))?;
    let ts_rank = ranks
        .get("tsRank")
        .and_then(Value::as_f64)
        .ok_or_else(|| RunnerError::new("INVALID_CURSOR", "Invalid cursor: missing tsRank"))?;
    let last_record_ids_per_object = cursor
        .get("lastRecordIdsPerObject")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| {
            RunnerError::new(
                "INVALID_CURSOR",
                "Invalid cursor: missing lastRecordIdsPerObject",
            )
        })?;
    Ok(SearchCursor {
        ts_rank_cd,
        ts_rank,
        last_record_ids_per_object,
    })
}

fn object_priority(object_name: &str) -> u8 {
    match object_name {
        "person" => 5,
        "company" => 4,
        "opportunity" => 3,
        "note" => 2,
        "task" => 1,
        _ => 0,
    }
}

fn has_cjk_characters(value: &str) -> bool {
    static CJK_CHARACTER_PATTERN: OnceLock<Regex> = OnceLock::new();
    CJK_CHARACTER_PATTERN
        .get_or_init(|| {
            Regex::new(
                r"[\p{Script_Extensions=Han}\p{Script_Extensions=Hiragana}\p{Script_Extensions=Katakana}\p{Script_Extensions=Hangul}]",
            )
            .expect("valid CJK script pattern")
        })
        .is_match(value)
}

fn escape_for_ilike(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '%' | '_') {
            output.push('\\');
        }
        output.push(character);
    }
    output
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::query::runner::test_support::DB_TEST_LOCK;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn search_ranks_labels_and_paginates_with_the_upstream_cursor() -> Result<(), RunnerError>
    {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return Ok(());
        };
        let _guard = DB_TEST_LOCK.lock().await;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .map_err(RunnerError::database)?;
        let workspace_id = Uuid::new_v4();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let schema_sql = quote_identifier(&schema_name)?;
        let table_sql = format!("{}.{}", schema_sql, quote_identifier("event")?);
        let object_id = Uuid::new_v4();
        let id_field = crate::metadata::flat::FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: FieldMetadataType::Uuid,
            name: String::from("id"),
            label: String::from("id"),
            ..Default::default()
        };
        let name_field = crate::metadata::flat::FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: FieldMetadataType::Text,
            name: String::from("name"),
            label: String::from("name"),
            ..Default::default()
        };
        let search_field = crate::metadata::flat::FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: FieldMetadataType::TsVector,
            name: String::from(SEARCH_VECTOR_FIELD_NAME),
            label: String::from(SEARCH_VECTOR_FIELD_NAME),
            ..Default::default()
        };
        let fields = vec![id_field.clone(), name_field.clone(), search_field.clone()];
        let mut field_maps = crate::metadata::flat::FlatEntityMaps::default();
        for field in &fields {
            field_maps.add(
                field.id.to_string(),
                field.clone(),
                field.id.to_string(),
                workspace_id.to_string(),
            );
        }
        let object = FlatObjectMetadata {
            id: object_id,
            workspace_id,
            name_singular: String::from("event"),
            name_plural: String::from("events"),
            label_singular: String::from("Event"),
            label_plural: String::from("Events"),
            is_active: true,
            is_searchable: true,
            label_identifier_field_metadata_id: Some(name_field.id),
            target_table_name: Some(String::from("event")),
            field_ids: fields.iter().map(|field| field.id.to_string()).collect(),
            ..Default::default()
        };
        let schema = Arc::new(
            RunnerSchema::build(
                &workspace_id,
                std::slice::from_ref(&object),
                &field_maps,
                &crate::metadata::flat::FlatEntityMaps::default(),
            )
            .expect("search schema"),
        );
        sqlx::query(&format!("CREATE SCHEMA {}", schema_sql))
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;
        sqlx::query(&format!(
            "CREATE TABLE {} (id uuid PRIMARY KEY, name text NOT NULL, \"{}\" tsvector)",
            table_sql, SEARCH_VECTOR_FIELD_NAME
        ))
        .execute(&pool)
        .await
        .map_err(RunnerError::database)?;
        let first_id = Uuid::from_u128(1);
        let second_id = Uuid::from_u128(2);
        for (id, name) in [(&first_id, "Alice Alpha"), (&second_id, "Bob Alpha")] {
            sqlx::query(&format!(
                "INSERT INTO {} (id, name, \"{}\") VALUES ($1, $2, to_tsvector('simple', $2))",
                table_sql, SEARCH_VECTOR_FIELD_NAME
            ))
            .bind(id)
            .bind(name)
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;
        }
        let args = SearchArgs {
            search_input: String::from("Alpha"),
            limit: 1,
            after: None,
            included_object_name_singulars: Vec::new(),
            excluded_object_name_singulars: Vec::new(),
            filter: None,
            locale: String::from("en"),
        };
        let first = search(
            &pool,
            schema.clone(),
            std::slice::from_ref(&object),
            &args,
            2000,
            true,
        )
        .await
        .expect("first search page");
        assert_eq!(first["pageInfo"]["hasNextPage"], json!(true));
        assert_eq!(first["edges"][0]["node"]["label"], json!("Alice Alpha"));
        assert_eq!(
            first["edges"][0]["node"]["objectNameSingular"],
            json!("event")
        );
        let second = search(
            &pool,
            schema,
            &[object],
            &SearchArgs {
                after: first["pageInfo"]["endCursor"].as_str().map(str::to_string),
                ..args
            },
            2000,
            true,
        )
        .await
        .expect("second search page");
        assert_eq!(second["edges"][0]["node"]["label"], json!("Bob Alpha"));
        assert_eq!(second["pageInfo"]["hasNextPage"], json!(false));
        sqlx::query(&format!("DROP SCHEMA {} CASCADE", schema_sql))
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;
        assert_ne!(first_id, second_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn object_filtering_matches_upstream_inclusion_rules() {
        let object = |name: &str, active: bool, searchable: bool| FlatObjectMetadata {
            name_singular: name.to_string(),
            is_active: active,
            is_searchable: searchable,
            ..Default::default()
        };
        let objects = vec![
            object("person", true, true),
            object("company", true, false),
            object("task", false, true),
            object("message", true, true),
        ];
        let args = SearchArgs {
            search_input: String::new(),
            limit: 10,
            after: None,
            included_object_name_singulars: Vec::new(),
            excluded_object_name_singulars: Vec::new(),
            filter: None,
            locale: String::from("en"),
        };
        let defaults = select_objects(&objects, &args)
            .into_iter()
            .map(|object| object.name_singular)
            .collect::<Vec<_>>();
        assert_eq!(defaults, vec!["person", "message"]);

        let included = SearchArgs {
            included_object_name_singulars: vec![String::from("company"), String::from("message")],
            ..args.clone()
        };
        assert_eq!(
            select_objects(&objects, &included)
                .into_iter()
                .map(|object| object.name_singular)
                .collect::<Vec<_>>(),
            vec!["company"]
        );
        let excluded = SearchArgs {
            excluded_object_name_singulars: vec![String::from("person")],
            ..args
        };
        assert_eq!(
            select_objects(&objects, &excluded)
                .into_iter()
                .map(|object| object.name_singular)
                .collect::<Vec<_>>(),
            vec!["message"]
        );
    }

    #[test]
    fn unreadable_objects_do_not_abort_global_search() {
        let readable = FlatObjectMetadata {
            id: Uuid::new_v4(),
            name_singular: String::from("person"),
            ..Default::default()
        };
        let denied = FlatObjectMetadata {
            id: Uuid::new_v4(),
            name_singular: String::from("task"),
            ..Default::default()
        };
        let mut permissions = crate::permissions::RoleRecordsPermissions {
            objects: [(readable.id.to_string(), Default::default())]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        permissions
            .objects
            .get_mut(&readable.id.to_string())
            .expect("object permission")
            .can_read_all_records = true;
        permissions
            .objects
            .get_mut(&readable.id.to_string())
            .expect("object permission")
            .can_read_some_records = true;

        assert!(can_read_object(None, &readable));
        assert!(can_read_object(Some(&permissions), &readable));
        assert!(!can_read_object(Some(&permissions), &denied));
    }

    #[test]
    fn cursor_round_trip_preserves_ranks_and_per_object_ids() {
        let cursor = encode_cursor_data(&Map::from_iter([
            (
                String::from("lastRanks"),
                json!({"tsRankCD": 0.5, "tsRank": 0.25}),
            ),
            (
                String::from("lastRecordIdsPerObject"),
                json!({"person": "p1", "company": "c1"}),
            ),
        ]));
        let decoded = decode_search_cursor(&cursor).expect("cursor");
        assert_eq!(decoded.ts_rank_cd, 0.5);
        assert_eq!(decoded.ts_rank, 0.25);
        assert_eq!(decoded.last_record_ids_per_object["person"], "p1");
        assert_eq!(decoded.last_record_ids_per_object["company"], "c1");
    }

    #[test]
    fn cjk_detection_and_ilike_escaping_match_upstream() {
        assert!(has_cjk_characters("Cộng đồng 東京 한글"));
        assert!(has_cjk_characters("𠮷 神 ﾒ ぁ ᄀ"));
        assert!(!has_cjk_characters("plain ASCII"));
        assert_eq!(escape_for_ilike("50%_\\"), "50\\%\\_\\\\");
    }

    #[test]
    fn object_overrides_control_search_activity_and_labels() {
        let mut object = FlatObjectMetadata {
            name_singular: String::from("company"),
            label_singular: String::from("Company"),
            is_active: true,
            application_universal_identifier: String::from("owner"),
            overrides: Some(json!({
                "owner": {"isActive": false, "labelSingular": "Owner Company"},
                "custom": {"labelSingular": "Custom Company"}
            })),
            ..Default::default()
        };
        assert!(!effective_is_active(&object));
        assert_eq!(effective_label_singular(&object, "en"), "Custom Company");

        object.overrides = Some(json!({"isActive": false, "labelSingular": "Legacy Company"}));
        assert!(!effective_is_active(&object));
        assert_eq!(effective_label_singular(&object, "en"), "Legacy Company");
    }

    #[test]
    fn translated_labels_take_precedence_and_fall_back_to_the_rename() {
        let mut object = FlatObjectMetadata {
            label_singular: String::from("Company"),
            application_universal_identifier: String::from("owner"),
            overrides: Some(json!({
                "owner": {"labelSingular": "Owner Company"},
                "custom": {
                    "labelSingular": "Custom Company",
                    "translations": {"fr-FR": {"labelSingular": "Société"}}
                }
            })),
            ..Default::default()
        };
        assert_eq!(effective_label_singular(&object, "fr-FR"), "Société");
        assert_eq!(effective_label_singular(&object, "vi-VN"), "Custom Company");

        object.overrides = Some(json!({
            "labelSingular": "Legacy Company",
            "translations": {"vi-VN": {"labelSingular": "Công ty"}}
        }));
        assert_eq!(effective_label_singular(&object, "vi-VN"), "Công ty");
        assert_eq!(effective_label_singular(&object, "en"), "Legacy Company");
    }

    #[test]
    fn standard_label_catalog_matches_the_pinned_twenty_release() {
        assert_eq!(object_label_message_id("Company"), "Jk7iDd");
        assert_eq!(object_label_message_id("Person"), "n+OC6R");
        assert_eq!(
            standard_label_translation("Company", "fr-FR"),
            Some("Entreprise")
        );
        assert_eq!(
            standard_label_translation("Company", "vi-VN"),
            Some("Công ty")
        );
        assert_ne!(
            standard_label_translation("Company", "pseudo-en"),
            Some("Company")
        );
        assert_eq!(standard_label_translation("Unknown label", "fr-FR"), None);

        let catalog: Value =
            serde_json::from_str(SEARCH_OBJECT_LABEL_CATALOG).expect("valid label catalog");
        let locales = catalog.as_object().expect("locale catalog");
        assert_eq!(locales.len(), 35);
        assert!(locales.values().all(|entries| entries
            .as_object()
            .is_some_and(|entries| entries.len() == 39)));
    }

    #[test]
    fn standard_labels_follow_authored_translation_and_rename_precedence() {
        let mut object = FlatObjectMetadata {
            label_singular: String::from("Company"),
            application_universal_identifier: String::from("owner"),
            overrides: Some(json!({"owner": {"labelSingular": ""}})),
            ..Default::default()
        };
        assert_eq!(effective_label_singular(&object, "fr-FR"), "Entreprise");

        object.overrides = Some(json!({"owner": {"labelSingular": "Custom Company"}}));
        assert_eq!(effective_label_singular(&object, "fr-FR"), "Custom Company");

        object.overrides = Some(json!({
            "owner": {"translations": {"fr-FR": {"labelSingular": "Société"}}}
        }));
        assert_eq!(effective_label_singular(&object, "fr-FR"), "Société");

        object.overrides = Some(json!({
            "owner": {"translations": {"fr-FR": {"labelSingular": ""}}}
        }));
        assert_eq!(effective_label_singular(&object, "fr-FR"), "");
    }

    #[test]
    fn link_favicon_url_matches_the_upstream_hostname_lookup() {
        assert_eq!(
            link_favicon_url("https://www.example.com/path").as_deref(),
            Some("https://twenty-icons.com/example.com")
        );
        assert_eq!(
            link_favicon_url("example.com/").as_deref(),
            Some("https://twenty-icons.com/example.com")
        );
        assert_eq!(link_favicon_url("   "), None);
        assert_eq!(link_favicon_url("https://"), None);
    }

    #[test]
    fn malformed_cursor_fails_with_the_upstream_code() {
        let error = decode_search_cursor("not-a-cursor").expect_err("invalid cursor");
        assert_eq!(error.code(), "INVALID_CURSOR");
    }
}
