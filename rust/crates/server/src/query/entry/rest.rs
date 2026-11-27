use std::collections::HashMap;

use axum::extract::FromRequestParts;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::Request;
use axum::extract::State;
use axum::http::Method;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::any;
use axum::Json;
use axum::Router;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::middleware::AuthContext;
use crate::metadata::api::service::load_flat_maps;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FlatWorkspaceMaps;
use crate::metadata::flat::RelationType;
use crate::metadata::names::camel_case;
use crate::metadata::names::pascal_case;
use crate::permissions::records_permissions_for_auth;
use crate::permissions::PermissionError;
use crate::query::cursor::decode_cursor;
use crate::query::runner::aggregate::Aggregate;
use crate::query::runner::aggregate::AggregateOperation;
use crate::query::runner::create_many;
use crate::query::runner::create_one;
use crate::query::runner::find_many;
use crate::query::runner::find_one;
use crate::query::runner::find_one::FindOneArgs;
use crate::query::runner::group_by;
use crate::query::runner::require_filtered_mutation;
use crate::query::runner::run_mutation;
use crate::query::runner::run_mutation_one;
use crate::query::runner::ActorContext;
use crate::query::runner::CreateArgs;
use crate::query::runner::FilteredMutationKind;
use crate::query::runner::FindManyArgs;
use crate::query::runner::Granularity;
use crate::query::runner::GroupByDefinition;
use crate::query::runner::NestedRelationSelection;
use crate::query::runner::RunnerSchema;
use crate::state::AppState;

use super::rest_parser::coerce_filter_values;
use super::rest_parser::parse_aggregates;
use super::rest_parser::parse_bool;
use super::rest_parser::parse_depth;
use super::rest_parser::parse_fields;
use super::rest_parser::parse_filter;
use super::rest_parser::parse_limit;
use super::rest_parser::parse_order_by;
use super::rest_parser::pick_fields;
use super::schema::WorkspaceSchema;

pub const DEFAULT_PAGE_SIZE: usize = 60;

pub const MAX_PAGE_SIZE: usize = 200;

pub fn router() -> Router<AppState> {
    Router::new().route("/{*path}", any(rest_endpoint))
}

#[derive(Debug, Clone)]
pub struct RestError {
    pub status: StatusCode,
    pub error: &'static str,
    pub messages: Vec<String>,
    pub code: &'static str,
    include_code: bool,
}

impl RestError {
    pub fn new(
        status: StatusCode,
        error: &'static str,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            error,
            messages: vec![message.into()],
            code,
            include_code: false,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "BadRequestException",
            "BAD_REQUEST",
            message,
        )
    }

    pub fn invalid_limit(message: impl Into<String>) -> Self {
        Self::bad_request(message).with_code("INVALID_LIMIT_QUERY_PARAM")
    }

    pub fn invalid_depth(message: impl Into<String>) -> Self {
        Self::bad_request(message).with_code("INVALID_DEPTH_QUERY_PARAM")
    }

    pub fn invalid_fields(message: impl Into<String>) -> Self {
        Self::bad_request(message).with_code("INVALID_FIELDS_QUERY_PARAM")
    }

    pub fn invalid_filter(message: impl Into<String>) -> Self {
        Self::bad_request(message).with_code("INVALID_FILTER_QUERY_PARAM")
    }

    pub fn invalid_order_by(message: impl Into<String>) -> Self {
        Self::bad_request(message).with_code("INVALID_ORDER_BY_QUERY_PARAM")
    }

    pub fn invalid_group_by(message: impl Into<String>) -> Self {
        Self::bad_request(message).with_code("INVALID_GROUP_BY_QUERY_PARAM")
    }

    pub fn invalid_aggregates(message: impl Into<String>) -> Self {
        Self::bad_request(message).with_code("INVALID_AGGREGATE_FIELDS_QUERY_PARAM")
    }

    pub fn invalid_cursor(message: impl Into<String>) -> Self {
        Self::bad_request(message).with_code("INVALID_CURSOR")
    }

    pub fn record_not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "NotFoundException",
            "RECORD_NOT_FOUND",
            "Record not found",
        )
    }

    pub fn method_not_allowed() -> Self {
        Self::new(
            StatusCode::METHOD_NOT_ALLOWED,
            "MethodNotAllowedException",
            "METHOD_NOT_ALLOWED",
            "Method not supported.",
        )
    }

    pub fn unknown_object(name: &str) -> Self {
        Self::bad_request(format!("object '{name}' not found. eg: companies"))
    }

    pub fn with_code(mut self, code: &'static str) -> Self {
        self.code = code;
        self
    }

    pub fn with_response_code(mut self) -> Self {
        self.include_code = true;
        self
    }

    pub fn response(&self) -> Response {
        let mut payload = json!({
            "statusCode": self.status.as_u16(),
            "error": self.error,
            "messages": self.messages,
        });
        if self.include_code {
            payload["code"] = json!(self.code);
        }
        (self.status, Json(payload)).into_response()
    }
}

fn status_error(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "BadRequestException",
        StatusCode::UNAUTHORIZED => "UnauthorizedException",
        StatusCode::FORBIDDEN => "ForbiddenException",
        StatusCode::NOT_FOUND => "NotFoundException",
        StatusCode::METHOD_NOT_ALLOWED => "MethodNotAllowedException",
        StatusCode::CONFLICT => "ConflictException",
        _ => "InternalServerErrorException",
    }
}

async fn rest_endpoint(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(parameters): Query<HashMap<String, String>>,
    request: Request,
) -> Response {
    let (mut parts, body) = request.into_parts();
    let auth = match AuthContext::from_request_parts(&mut parts, &state).await {
        Ok(context) => context,
        Err(error) => {
            let status = StatusCode::from_u16(error.status).unwrap_or(StatusCode::UNAUTHORIZED);

            return RestError::new(
                status,
                status_error(status),
                error.code,
                error.messages.join(", "),
            )
            .response();
        }
    };

    if auth.agent_id.is_some() {
        return RestError::new(
            StatusCode::UNAUTHORIZED,
            "UnauthorizedException",
            "INVALID_AUTH_CONTEXT",
            "This token is only accepted by MCP",
        )
        .response();
    }

    let Some(workspace_id) = auth.workspace_id else {
        return RestError::new(
            StatusCode::UNAUTHORIZED,
            "UnauthorizedException",
            "INVALID_AUTH_CONTEXT",
            "Could not retrieve workspace ID",
        )
        .response();
    };

    let maps = match load_flat_maps(&state, workspace_id).await {
        Ok(maps) => maps,
        Err(error) => return RestError::internal(error.message.clone()).response(),
    };
    let schema = WorkspaceSchema::build(&maps);
    let permissions = match records_permissions_for_auth(&state, &auth).await {
        Ok(permissions) => permissions,
        Err(error) => return rest_permission_error(error).response(),
    };
    let runner_schema = match permissions {
        Some(permissions) => match RunnerSchema::build_for_rest_with_permissions(
            &workspace_id,
            schema.objects(),
            &maps.field_metadata_maps,
            &maps.index_metadata_maps,
            permissions,
        ) {
            Ok(schema) => schema,
            Err(error) => return RestError::internal(error.message().to_string()).response(),
        },
        None => match RunnerSchema::build(
            &workspace_id,
            schema.objects(),
            &maps.field_metadata_maps,
            &maps.index_metadata_maps,
        ) {
            Ok(schema) => schema,
            Err(error) => return RestError::internal(error.message().to_string()).response(),
        },
    };
    let path_parts = path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if path_parts.is_empty() {
        return RestError::bad_request("A REST object path is required").response();
    }

    let actor = ActorContext::manual(auth.workspace_member_id.map(|id| id.to_string()), None);
    let body = if matches!(
        parts.method,
        Method::POST | Method::PATCH | Method::PUT | Method::DELETE
    ) {
        let bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return RestError::bad_request("Request body was too large or unreadable")
                    .response()
            }
        };

        if bytes.is_empty() {
            Value::Object(Map::new())
        } else {
            match serde_json::from_slice::<Value>(&bytes) {
                Ok(Value::Object(object)) => Value::Object(object),
                Ok(_) => {
                    return RestError::bad_request("Request body must be an object").response()
                }
                Err(_) => {
                    return RestError::bad_request("Request body must be valid JSON").response()
                }
            }
        }
    } else {
        Value::Object(Map::new())
    };

    match dispatch_rest(
        &state,
        &maps,
        &schema,
        &runner_schema,
        &parts.method,
        &path_parts,
        &parameters,
        body,
        actor,
    )
    .await
    {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

impl RestError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "InternalServerErrorException",
            "INTERNAL_SERVER_ERROR",
            message,
        )
    }
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_rest(
    state: &AppState,
    _maps: &FlatWorkspaceMaps,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    method: &Method,
    path: &[&str],
    parameters: &HashMap<String, String>,
    body: Value,
    actor: ActorContext,
) -> Result<Response, RestError> {
    let reserved = path.get(1).copied().unwrap_or_default();

    if reserved == "batch" {
        if *method != Method::POST {
            return Err(RestError::method_not_allowed());
        }

        let object = path[0];
        let records = body
            .get("data")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_object)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .ok_or_else(|| RestError::bad_request("createMany requires data"))?;
        let ids = create_many(
            &state.core_pool,
            runner_schema,
            object,
            &CreateArgs {
                records,
                upsert: parse_bool(parameter(parameters, "upsert"))?.unwrap_or(false),
                backfill_position_undefined: true,
                actor: Some(actor.clone()),
            },
        )
        .await
        .map_err(rest_runner_error)?;
        let records = records_by_ids(state, runner_schema, object, &ids, false, &[]).await?;
        let plural = schema
            .object(object)
            .map(|object| object.name_plural.clone())
            .unwrap_or_else(|| object.to_string());
        let response = format_mutation_array(&pascal_case(&plural), records);

        return Ok((StatusCode::CREATED, Json(response)).into_response());
    }

    let object = path[0];
    let object_model = rest_object(schema, object)?;
    let object = object_model.name_singular.as_str();
    let depth = parse_depth(parameter(parameters, "depth"))?;
    let fields = parse_fields(parameter(parameters, "fields"))?;
    let (picked_fields, relations) =
        resolve_rest_fields(schema, runner_schema, object, fields.as_deref(), depth)?;
    let nested = nested_from_names(runner_schema, object, &relations);

    if path.len() > 3 || (path.len() == 3 && (*method != Method::PATCH || path[2] != "restore")) {
        return Err(RestError::bad_request(format!(
            "Query path '/rest/{}' invalid. Valid examples: /rest/companies/id or /rest/companies or /rest/batch/companies",
            path.join("/")
        )));
    }

    if let Some(id_or_action) = path.get(1).copied() {
        match id_or_action {
            "groupBy" if *method == Method::GET => {
                let definitions =
                    parse_group_definitions(schema, object, parameter(parameters, "group_by"))?;
                let filter = coerce_filter(parameter(parameters, "filter"), object_model)?;
                let aggregate_names = parse_aggregates(parameter(parameters, "aggregate"))?;
                let mut aggregates = vec![Aggregate::total_count()];

                for name in aggregate_names {
                    if let Some(parsed) = aggregate_from_name(name)? {
                        aggregates.push(parsed);
                    }
                }

                let groups = group_by(
                    &state.core_pool,
                    runner_schema,
                    object,
                    filter.as_ref(),
                    parse_bool(parameter(parameters, "soft_delete"))?.unwrap_or(false),
                    &definitions,
                    &aggregates,
                    parse_limit(parameter(parameters, "limit"))?.or(Some(DEFAULT_PAGE_SIZE)),
                    parameter(parameters, "timeZone"),
                )
                .await
                .map_err(rest_runner_error)?;
                let output = groups
                    .into_iter()
                    .map(|row| {
                        let mut record = Map::new();
                        record.insert(String::from("groupByDimensionValues"), row.group_key);

                        for (name, value) in row.values {
                            record.insert(name, value);
                        }

                        Value::Object(record)
                    })
                    .collect::<Vec<_>>();

                return Ok(Json(Value::Array(output)).into_response());
            }
            "duplicates" => {
                return Err(RestError::bad_request(
                    "findDuplicates requires duplicate criteria execution in a follow-on slice",
                ));
            }
            "merge" => {
                return Err(RestError::bad_request(
                    "mergeMany is deferred to the record merge slice",
                ));
            }
            "restore" if *method == Method::PATCH => {
                return restore_records(
                    state,
                    runner_schema,
                    object,
                    None,
                    parameters,
                    &body,
                    actor,
                    picked_fields.as_deref(),
                    &nested,
                )
                .await;
            }
            id => {
                if let Some(action) = path.get(2).copied() {
                    if action == "restore" {
                        return restore_records(
                            state,
                            runner_schema,
                            object,
                            Some(id),
                            parameters,
                            &body,
                            actor,
                            picked_fields.as_deref(),
                            &nested,
                        )
                        .await;
                    }

                    return Err(RestError::bad_request(format!(
                        "Query path '/rest/{}' invalid. Valid examples: /rest/companies/id or /rest/companies or /rest/batch/companies",
                        path.join("/")
                    )));
                }

                let id = parse_path_uuid(id)?;

                return id_operation(
                    state,
                    schema,
                    runner_schema,
                    object_model,
                    method,
                    parameters,
                    body,
                    actor,
                    object,
                    id,
                    picked_fields,
                    &nested,
                )
                .await;
            }
        }
    }

    match *method {
        Method::GET => {
            if parameter(parameters, "starting_after").is_some()
                && parameter(parameters, "ending_before").is_some()
            {
                return Err(RestError::invalid_cursor(
                    "Cannot provide both before and after",
                ));
            }

            let order_by = parse_order_by(parameter(parameters, "order_by"))?;
            validate_order_by(schema, object, order_by.as_ref())?;
            let filter = coerce_filter(parameter(parameters, "filter"), object_model)?;
            let args = FindManyArgs {
                filter,
                order_by,
                first: parse_limit(parameter(parameters, "limit"))?
                    .or(Some(DEFAULT_PAGE_SIZE))
                    .map(|value| value.min(MAX_PAGE_SIZE)),
                after: optional_cursor(parameter(parameters, "starting_after"))?,
                before: optional_cursor(parameter(parameters, "ending_before"))?,
                selected_field_names: fields.clone().unwrap_or_default(),
                ..Default::default()
            };
            let result = find_many(
                &state.core_pool,
                runner_schema,
                object,
                &args,
                true,
                &nested,
            )
            .await
            .map_err(rest_runner_error)?;
            let mut records = result.records;

            for record in &mut records {
                if let Some(record) = record.as_object_mut() {
                    pick_fields(record, picked_fields.as_deref());
                }
            }

            Ok(Json(json!({
                "data": { camel_case(object_model.name_plural.as_str()): records },
                "totalCount": result.total_count,
                "pageInfo": {
                    "startCursor": result.page_info.start_cursor,
                    "endCursor": result.page_info.end_cursor,
                    "hasNextPage": result.page_info.has_next_page,
                    "hasPreviousPage": result.page_info.has_previous_page,
                },
            }))
            .into_response())
        }
        Method::POST => {
            let data = body.get("data").cloned().unwrap_or(body.clone());
            let Some(data) = data.as_object().cloned() else {
                return Err(RestError::bad_request("createOne requires data"));
            };
            let id = create_one(
                &state.core_pool,
                runner_schema,
                object,
                data,
                parse_bool(parameter(parameters, "upsert"))?.unwrap_or(false),
                true,
                Some(&actor),
            )
            .await
            .map_err(rest_runner_error)?;
            let mut records = records_by_ids(
                state,
                runner_schema,
                object,
                &[id],
                false,
                &nested_from_names(runner_schema, object, &relations),
            )
            .await?;
            let mut record = records.remove(0);

            if let Some(record) = record.as_object_mut() {
                pick_fields(record, picked_fields.as_deref());
            }

            let key = format!("create{}", pascal_case(object));
            Ok((
                StatusCode::CREATED,
                Json(json!({ "data": { key: record } })),
            )
                .into_response())
        }
        Method::PATCH | Method::PUT => {
            let filter =
                coerce_filter(parameter(parameters, "filter"), object_model)?.ok_or_else(|| {
                    RestError::bad_request("A non-empty filter is required for a bulk mutation")
                })?;
            let data = body
                .get("data")
                .cloned()
                .ok_or_else(|| RestError::bad_request("updateMany requires data"))?;
            require_filtered_mutation(
                &state.core_pool,
                runner_schema,
                object,
                FilteredMutationKind::Update,
                Some(filter.clone()),
                Some(data),
                Some(&actor),
                false,
                vec![String::from("id")],
            )
            .await
            .map_err(rest_runner_error)?;
            let mut records = find_many(
                &state.core_pool,
                runner_schema,
                object,
                &FindManyArgs {
                    filter: Some(filter),
                    selected_field_names: fields.clone().unwrap_or_default(),
                    ..Default::default()
                },
                false,
                &nested_from_names(runner_schema, object, &relations),
            )
            .await
            .map_err(rest_runner_error)?
            .records;

            for record in &mut records {
                if let Some(record) = record.as_object_mut() {
                    pick_fields(record, picked_fields.as_deref());
                }
            }

            let plural = object_model.name_plural.as_str();
            let key = format!("update{}", pascal_case(plural));
            Ok(Json(json!({ "data": { key: records } })).into_response())
        }
        Method::DELETE => {
            let soft_delete = parse_bool(parameter(parameters, "soft_delete"))?.unwrap_or(false);
            let filter = coerce_filter(parameter(parameters, "filter"), object_model)?
                .ok_or_else(|| {
                    RestError::bad_request(if soft_delete {
                        "Filters are mandatory for bulk delete operations. Please provide at least one filter to prevent accidental deletion of all records."
                    } else {
                        "Filters are mandatory for bulk destroy operations. Please provide at least one filter to prevent accidental deletion of all records."
                    })
                })?;

            if filter.as_object().is_none_or(Map::is_empty) {
                return Err(RestError::bad_request(if soft_delete {
                    "Filters are mandatory for bulk delete operations. Please provide at least one filter to prevent accidental deletion of all records."
                } else {
                    "Filters are mandatory for bulk destroy operations. Please provide at least one filter to prevent accidental deletion of all records."
                }));
            }

            let kind = if soft_delete {
                FilteredMutationKind::SoftDelete
            } else {
                FilteredMutationKind::Destroy
            };
            let results = run_mutation(
                &state.core_pool,
                runner_schema,
                object,
                kind,
                Some(filter),
                None,
                Some(&actor),
                false,
                vec![String::from("id")],
            )
            .await
            .map_err(rest_runner_error)?;
            let ids = results
                .into_iter()
                .filter_map(|record| record.get("id").cloned())
                .collect::<Vec<_>>();
            let key = format!("delete{}", pascal_case(object));

            Ok(Json(json!({ "data": { key: ids } })).into_response())
        }
        _ => Err(RestError::method_not_allowed()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn id_operation(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    object_model: &crate::metadata::flat::FlatObjectMetadata,
    method: &Method,
    parameters: &HashMap<String, String>,
    body: Value,
    actor: ActorContext,
    object: &str,
    id: Uuid,
    picked_fields: Option<Vec<String>>,
    nested: &[NestedRelationSelection],
) -> Result<Response, RestError> {
    match *method {
        Method::GET => {
            let filter = Some(id_filter(id));
            let record = find_one(
                &state.core_pool,
                runner_schema,
                object,
                &FindOneArgs {
                    filter,
                    ..Default::default()
                },
                nested,
            )
            .await
            .map_err(rest_runner_error)?
            .ok_or_else(RestError::record_not_found)?;
            let mut record = record;

            if let Some(record) = record.as_object_mut() {
                pick_fields(record, picked_fields.as_deref());
            }

            let key = camel_case(object);
            Ok(Json(json!({ "data": { key: record } })).into_response())
        }
        Method::PATCH | Method::PUT => {
            let data = body
                .get("data")
                .cloned()
                .ok_or_else(|| RestError::bad_request("updateOne requires data"))?;
            run_mutation_one(
                &state.core_pool,
                runner_schema,
                object,
                FilteredMutationKind::Update,
                Some(id_filter(id)),
                Some(data),
                Some(&actor),
                false,
                vec![String::from("id")],
            )
            .await
            .map_err(rest_runner_error)?;
            let mut records =
                records_by_ids(state, runner_schema, object, &[id], false, nested).await?;
            let mut record = records.remove(0);

            if let Some(record) = record.as_object_mut() {
                pick_fields(record, picked_fields.as_deref());
            }

            let key = format!("update{}", pascal_case(object));
            Ok(Json(json!({ "data": { key: record } })).into_response())
        }
        Method::DELETE => {
            let soft_delete = parse_bool(parameter(parameters, "soft_delete"))?.unwrap_or(false);
            records_by_ids(state, runner_schema, object, &[id], true, &[])
                .await?
                .into_iter()
                .next()
                .ok_or_else(RestError::record_not_found)?;
            run_mutation_one(
                &state.core_pool,
                runner_schema,
                object,
                if soft_delete {
                    FilteredMutationKind::SoftDelete
                } else {
                    FilteredMutationKind::Destroy
                },
                Some(id_filter(id)),
                None,
                Some(&actor),
                true,
                vec![String::from("id")],
            )
            .await
            .map_err(rest_runner_error)?;
            let key = format!("delete{}", pascal_case(object));
            Ok(Json(json!({ "data": { key: { "id": id.to_string() } } })).into_response())
        }
        _ => {
            let _ = object_model;
            let _ = schema;
            Err(RestError::method_not_allowed())
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn restore_records(
    state: &AppState,
    runner_schema: &RunnerSchema,
    object: &str,
    id: Option<&str>,
    parameters: &HashMap<String, String>,
    body: &Value,
    actor: ActorContext,
    picked_fields: Option<&[String]>,
    nested: &[NestedRelationSelection],
) -> Result<Response, RestError> {
    if let Some(id) = id {
        let id = parse_path_uuid(id)?;
        run_mutation_one(
            &state.core_pool,
            runner_schema,
            object,
            FilteredMutationKind::Restore,
            Some(id_filter(id)),
            None,
            Some(&actor),
            true,
            vec![String::from("id")],
        )
        .await
        .map_err(rest_runner_error)?;
        let mut records =
            records_by_ids(state, runner_schema, object, &[id], false, nested).await?;
        let mut record = records.remove(0);

        if let Some(record) = record.as_object_mut() {
            pick_fields(record, picked_fields);
        }

        let key = format!("restore{}", pascal_case(object));

        return Ok(Json(json!({ "data": { key: record } })).into_response());
    }

    let filter = body
        .get("filter")
        .cloned()
        .filter(|value| !value.is_null())
        .or_else(|| {
            serde_json::from_str::<Value>(parameter(parameters, "filter").unwrap_or("null"))
                .ok()
                .filter(|value| !value.is_null())
        })
        .ok_or_else(|| {
            RestError::bad_request("A non-empty filter is required for a bulk mutation")
        })?;

    if filter.as_object().is_none_or(Map::is_empty) {
        return Err(RestError::bad_request(
            "A non-empty filter is required for a bulk mutation",
        ));
    }

    let results = run_mutation(
        &state.core_pool,
        runner_schema,
        object,
        FilteredMutationKind::Restore,
        Some(filter),
        None,
        Some(&actor),
        true,
        vec![String::from("id")],
    )
    .await
    .map_err(rest_runner_error)?;
    let ids = results
        .into_iter()
        .filter_map(|record| record.get("id").and_then(Value::as_str).map(str::to_string))
        .collect::<Vec<_>>();
    let parsed_ids = ids
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect::<Vec<_>>();
    let mut records = records_by_ids(state, runner_schema, object, &parsed_ids, false, nested)
        .await
        .unwrap_or_default();

    for record in &mut records {
        if let Some(record) = record.as_object_mut() {
            pick_fields(record, picked_fields);
        }
    }

    let plural = plural_of(runner_schema, object);
    let key = format!("restore{}", pascal_case(&plural));

    Ok(Json(json!({ "data": { key: records } })).into_response())
}

fn plural_of(runner_schema: &RunnerSchema, object: &str) -> String {
    runner_schema
        .object(object)
        .ok()
        .map(|model| model.object.name_plural.clone())
        .unwrap_or_else(|| object.to_string())
}

fn parse_path_uuid(raw: &str) -> Result<Uuid, RestError> {
    let parsed = Uuid::parse_str(raw)
        .ok()
        .filter(|uuid| uuid.get_version_num() == 4);

    parsed.ok_or_else(|| RestError::bad_request(format!("'{raw}' is not a valid UUID")))
}

fn parameter<'a>(parameters: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    parameters.get(name).map(String::as_str)
}

fn coerce_filter(
    raw: Option<&str>,
    object_model: &crate::metadata::flat::FlatObjectMetadata,
) -> Result<Option<Value>, RestError> {
    let mut filter = parse_filter(raw)?;

    if let Some(filter) = filter.as_mut() {
        coerce_filter_values(filter);
    }

    let _ = object_model;
    Ok(filter)
}

fn optional_cursor(raw: Option<&str>) -> Result<Option<String>, RestError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    decode_cursor(raw)
        .map(|_| Some(raw.to_string()))
        .map_err(|error| RestError::invalid_cursor(error.message()))
}

fn parse_group_definitions(
    schema: &WorkspaceSchema,
    object: &str,
    raw: Option<&str>,
) -> Result<Vec<GroupByDefinition>, RestError> {
    let definitions = super::rest_parser::parse_group_by(raw)?;

    if definitions.is_empty() {
        return Err(RestError::invalid_group_by(
            "group_by is mandatory for the groupBy endpoint",
        ));
    }

    definitions
        .into_iter()
        .map(|(field, granularity)| {
            let _model = schema
                .object(object)
                .ok_or_else(|| RestError::unknown_object(object))?;
            let field_type = schema
                .fields(object)
                .iter()
                .find(|metadata| metadata.name == field)
                .map(|field| field.field_type)
                .ok_or_else(|| RestError::bad_request(format!("Unknown group_by field {field}")))?;
            let granularity = match granularity.as_deref() {
                None => Granularity::None,
                Some("NONE") => Granularity::None,
                Some("DAY") => Granularity::Day,
                Some("WEEK") => Granularity::Week,
                Some("MONTH") => Granularity::Month,
                Some("QUARTER") => Granularity::Quarter,
                Some("YEAR") => Granularity::Year,
                Some("DAY_OF_THE_WEEK") => Granularity::DayOfTheWeek,
                Some("MONTH_OF_THE_YEAR") => Granularity::MonthOfTheYear,
                Some(_) => {
                    return Err(RestError::invalid_group_by(
                        "The group_by query param is invalid",
                    ))
                }
            };

            if granularity != Granularity::None
                && !matches!(
                    field_type,
                    FieldMetadataType::Date | FieldMetadataType::DateTime
                )
            {
                return Err(RestError::invalid_group_by(
                    "A granularity is only valid for date fields",
                ));
            }

            Ok(GroupByDefinition {
                field_name: field,
                sub_field_name: None,
                granularity,
            })
        })
        .collect()
}

fn aggregate_from_name(name: Value) -> Result<Option<Aggregate>, RestError> {
    let Some(name) = name.as_str() else {
        return Err(RestError::invalid_aggregates(
            "The aggregate query param is invalid",
        ));
    };

    if name == "totalCount" {
        return Ok(Some(Aggregate::total_count()));
    }

    let (prefix, operation) = [
        ("countUniqueValues", AggregateOperation::CountUniqueValues),
        ("percentageNotEmpty", AggregateOperation::PercentageNotEmpty),
        ("percentageEmpty", AggregateOperation::PercentageEmpty),
        ("countNotEmpty", AggregateOperation::CountNotEmpty),
        ("countEmpty", AggregateOperation::CountEmpty),
        ("countTrue", AggregateOperation::CountTrue),
        ("countFalse", AggregateOperation::CountFalse),
        ("min", AggregateOperation::Min),
        ("max", AggregateOperation::Max),
        ("avg", AggregateOperation::Average),
        ("sum", AggregateOperation::Sum),
    ]
    .into_iter()
    .find(|(prefix, _)| name.starts_with(prefix))
    .ok_or_else(|| RestError::invalid_aggregates("The aggregate query param is invalid"))?;
    let raw_field = &name[prefix.len()..];
    let mut field = String::with_capacity(raw_field.len());

    for (index, character) in raw_field.chars().enumerate() {
        if index == 0 {
            field.extend(character.to_lowercase());
        } else {
            field.push(character);
        }
    }

    if field.is_empty() {
        return Err(RestError::invalid_aggregates(
            "The aggregate query param is invalid",
        ));
    }

    Ok(Some(Aggregate {
        operation,
        field_name: Some(field),
        sub_field_name: None,
    }))
}

fn id_filter(id: Uuid) -> Value {
    json!({ "id": { "eq": id.to_string() } })
}

fn validate_order_by(
    schema: &WorkspaceSchema,
    object: &str,
    order_by: Option<&Value>,
) -> Result<(), RestError> {
    let Some(entries) = order_by.and_then(Value::as_array) else {
        return Ok(());
    };

    for entry in entries {
        validate_order_by_value(schema, object, entry, None)?;
    }

    Ok(())
}

fn validate_order_by_value(
    schema: &WorkspaceSchema,
    object: &str,
    value: &Value,
    parent_composite: Option<FieldMetadataType>,
) -> Result<(), RestError> {
    let Some(entries) = value.as_object() else {
        return Ok(());
    };

    for (key, nested) in entries {
        if matches!(
            nested.as_str(),
            Some("AscNullsFirst")
                | Some("AscNullsLast")
                | Some("DescNullsFirst")
                | Some("DescNullsLast")
        ) {
            let known = match parent_composite {
                Some(composite) => {
                    crate::workspace_migrations::column_defs::composite_types::composite_properties(
                        crate::workspace_migrations::runner::ddl_field_type(composite),
                    )
                    .iter()
                    .any(|property| property.name == key)
                }
                None => schema.fields(object).iter().any(|field| field.name == *key),
            };

            if !known {
                return Err(RestError::invalid_order_by(format!(
                    "Field \"{key}\" does not exist or is not sortable"
                )));
            }

            continue;
        }

        let Some(field) = schema
            .fields(object)
            .iter()
            .find(|field| field.name == *key)
        else {
            return Err(RestError::invalid_order_by(format!(
                "Field \"{key}\" does not exist or is not sortable"
            )));
        };

        if !matches!(
            field.field_type,
            FieldMetadataType::FullName
                | FieldMetadataType::Links
                | FieldMetadataType::Emails
                | FieldMetadataType::Phones
                | FieldMetadataType::Address
                | FieldMetadataType::Currency
                | FieldMetadataType::Actor
        ) {
            return Err(RestError::invalid_order_by(format!(
                "Field \"{key}\" does not exist or is not sortable"
            )));
        }

        validate_order_by_value(schema, object, nested, Some(field.field_type))?;
    }

    Ok(())
}

fn nested_from_names(
    runner_schema: &RunnerSchema,
    object: &str,
    relations: &[String],
) -> Vec<NestedRelationSelection> {
    relations
        .iter()
        .map(|relation| NestedRelationSelection {
            relation_field_name: relation.clone(),
            selected_field_names: Vec::new(),
            first: None,
        })
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|relation| {
            runner_schema
                .object(object)
                .map(|model| {
                    model
                        .shape
                        .relation(&relation.relation_field_name)
                        .is_some()
                })
                .unwrap_or(false)
        })
        .collect()
}

async fn records_by_ids(
    state: &AppState,
    runner_schema: &RunnerSchema,
    object: &str,
    ids: &[Uuid],
    with_deleted: bool,
    nested: &[NestedRelationSelection],
) -> Result<Vec<Value>, RestError> {
    let filters = ids.iter().map(|id| id_filter(*id)).collect::<Vec<_>>();
    let mut records = find_many(
        &state.core_pool,
        runner_schema,
        object,
        &FindManyArgs {
            filter: Some(json!({ "or": filters })),
            with_deleted,
            ..Default::default()
        },
        false,
        nested,
    )
    .await
    .map_err(rest_runner_error)?
    .records;

    records.sort_by_key(|record| {
        ids.iter()
            .position(|id| {
                record.get("id").and_then(Value::as_str) == Some(id.to_string().as_str())
            })
            .unwrap_or(usize::MAX)
    });

    Ok(records)
}

fn format_mutation_array(key: &str, records: Vec<Value>) -> Value {
    json!({ "data": { key: records } })
}

fn rest_object<'a>(
    schema: &'a WorkspaceSchema,
    path_name: &str,
) -> Result<&'a crate::metadata::flat::FlatObjectMetadata, RestError> {
    schema
        .objects()
        .iter()
        .find(|object| object.name_plural == path_name || object.name_singular == path_name)
        .ok_or_else(|| RestError::unknown_object(path_name))
}

fn rest_permission_error(error: PermissionError) -> RestError {
    let status =
        StatusCode::from_u16(error.code.rest_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    RestError::new(status, "Error", error.code.as_str(), error.message).with_response_code()
}

fn rest_runner_error(error: crate::query::runner::RunnerError) -> RestError {
    match error.code() {
        "RECORD_NOT_FOUND" => RestError::record_not_found(),
        "PERMISSION_DENIED" | "RECORD_NOT_EDITABLE" => RestError::new(
            StatusCode::FORBIDDEN,
            "Error",
            error.code(),
            error.message(),
        )
        .with_response_code(),
        "INVALID_AUTH_CONTEXT" => RestError::new(
            StatusCode::UNAUTHORIZED,
            "UnauthorizedException",
            "INVALID_AUTH_CONTEXT",
            error.message(),
        ),
        "UNKNOWN_OBJECT" => RestError::unknown_object(error.message()),
        code if matches!(
            code,
            "ARGS_CONFLICT"
                | "INVALID_ARGS_FIRST"
                | "INVALID_ARGS_LAST"
                | "INVALID_QUERY_INPUT"
                | "INVALID_ARGS_DATA"
                | "INVALID_ARGS_FILTER"
                | "UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT"
                | "INVALID_CURSOR"
                | "TOO_MANY_RECORDS_TO_UPDATE"
                | "BAD_REQUEST"
                | "TOO_COMPLEX_QUERY"
                | "MISSING_TIMEZONE_FOR_DATE_GROUP_BY"
                | "INVALID_TIMEZONE"
                | "UNKNOWN_COLUMN"
                | "UNKNOWN_RELATION"
                | "UNKNOWN_SUB_FIELD"
                | "INVALID_DIRECTION"
        ) =>
        {
            RestError::bad_request(error.message()).with_code(code_static(code))
        }
        _ => RestError::internal(error.message()),
    }
}

fn code_static(code: &str) -> &'static str {
    match code {
        "ARGS_CONFLICT" => "ARGS_CONFLICT",
        "INVALID_ARGS_FIRST" => "INVALID_ARGS_FIRST",
        "INVALID_ARGS_LAST" => "INVALID_ARGS_LAST",
        "INVALID_QUERY_INPUT" => "INVALID_QUERY_INPUT",
        "INVALID_ARGS_DATA" => "INVALID_ARGS_DATA",
        "INVALID_ARGS_FILTER" => "INVALID_ARGS_FILTER",
        "UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT" => "UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT",
        "INVALID_CURSOR" => "INVALID_CURSOR",
        "TOO_MANY_RECORDS_TO_UPDATE" => "TOO_MANY_RECORDS_TO_UPDATE",
        "BAD_REQUEST" => "BAD_REQUEST",
        "TOO_COMPLEX_QUERY" => "TOO_COMPLEX_QUERY",
        "MISSING_TIMEZONE_FOR_DATE_GROUP_BY" => "MISSING_TIMEZONE_FOR_DATE_GROUP_BY",
        "INVALID_TIMEZONE" => "INVALID_TIMEZONE",
        "UNKNOWN_COLUMN" => "UNKNOWN_COLUMN",
        "UNKNOWN_RELATION" => "UNKNOWN_RELATION",
        "UNKNOWN_SUB_FIELD" => "UNKNOWN_SUB_FIELD",
        "INVALID_DIRECTION" => "INVALID_DIRECTION",
        _ => "BAD_REQUEST",
    }
}

fn resolve_rest_fields(
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    object: &str,
    fields: Option<&[String]>,
    depth: usize,
) -> Result<(Option<Vec<String>>, Vec<String>), RestError> {
    let Some(fields) = fields else {
        if depth == 0 {
            return Ok((None, Vec::new()));
        }

        let relations = schema
            .fields(object)
            .iter()
            .filter(|field| {
                matches!(
                    field.field_type,
                    FieldMetadataType::Relation | FieldMetadataType::MorphRelation
                )
            })
            .map(|field| field.name.clone())
            .collect::<Vec<_>>();

        return Ok((None, relations));
    };

    let metadata = schema.fields(object);
    let model = runner_schema.object(object).map_err(rest_runner_error)?;
    let mut invalid = Vec::new();
    let mut to_many_without_depth = Vec::new();
    let mut keys = Vec::new();
    let mut relations = Vec::new();

    for name in fields {
        let Some(field) = metadata.iter().find(|field| &field.name == name) else {
            invalid.push(name.clone());
            continue;
        };
        if runner_schema.is_field_unreadable(model.object.id, field.id) {
            invalid.push(name.clone());
            continue;
        }

        if matches!(
            field.field_type,
            FieldMetadataType::Relation | FieldMetadataType::MorphRelation
        ) {
            let relation = model.shape.relation(name);
            let is_to_many =
                relation.is_none_or(|relation| relation.relation_type != RelationType::ManyToOne);

            if is_to_many && depth == 0 {
                to_many_without_depth.push(name.clone());
                continue;
            }

            if let Some(join_column) =
                relation.and_then(|relation| relation.join_column_name.clone())
            {
                keys.push(join_column);
            }

            if depth >= 1 {
                keys.push(name.clone());
                relations.push(name.clone());
            }

            continue;
        }

        keys.push(name.clone());
    }

    if !invalid.is_empty() {
        return Err(RestError::invalid_fields(format!(
            "'fields' parameter invalid. Unknown or unreadable fields on '{}': {}",
            object,
            invalid.join(", ")
        )));
    }

    if !to_many_without_depth.is_empty() {
        return Err(RestError::invalid_fields(format!(
            "'fields' parameter invalid. One-to-many relation fields on '{}' require a positive depth: {}",
            object,
            to_many_without_depth.join(", ")
        )));
    }

    Ok((Some(keys), relations))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn response_json(error: RestError) -> Value {
        let response = error.response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("error response body");
        assert_eq!(status, error.status);
        serde_json::from_slice(&bytes).expect("error response json")
    }

    #[tokio::test]
    async fn permission_errors_include_the_upstream_code() {
        let body = response_json(rest_runner_error(crate::query::runner::RunnerError::new(
            "PERMISSION_DENIED",
            "Entity performing the request does not have permission",
        )))
        .await;
        assert_eq!(body["statusCode"], 403);
        assert_eq!(body["error"], "Error");
        assert_eq!(body["code"], "PERMISSION_DENIED");
        assert_eq!(
            body["messages"][0],
            "Entity performing the request does not have permission"
        );
    }

    #[tokio::test]
    async fn ordinary_rest_errors_keep_their_existing_shape() {
        let body = response_json(RestError::unknown_object("missing")).await;
        assert_eq!(
            body,
            json!({
                "statusCode": 400,
                "error": "BadRequestException",
                "messages": ["object 'missing' not found. eg: companies"],
            })
        );
        let body = response_json(
            RestError::invalid_fields("invalid").with_code("INVALID_FIELDS_QUERY_PARAM"),
        )
        .await;
        assert!(body.get("code").is_none());
    }
}
