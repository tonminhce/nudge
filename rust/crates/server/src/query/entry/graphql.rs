use std::collections::HashMap;

use axum::extract::FromRequestParts;
use axum::extract::Query;
use axum::extract::Request;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::error::AuthError;
use crate::auth::middleware::AuthContext;
use crate::metadata::api::errors::GraphQlError;
use crate::metadata::api::graphql::parse_request;
use crate::metadata::api::graphql::select_operation;
use crate::metadata::api::graphql::ExecutionRequest;
use crate::metadata::api::graphql::FieldSelection;
use crate::metadata::api::graphql::GraphqlRequestBody;
use crate::metadata::api::graphql::OperationKind;
use crate::metadata::api::service::load_flat_maps;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::names::camel_case;
use crate::metadata::names::pascal_case;
use crate::query::cursor::encode_cursor_data;
use crate::query::runner::create_many;
use crate::query::runner::create_one;
use crate::query::runner::find_many;
use crate::query::runner::find_one;
use crate::query::runner::find_one::FindOneArgs;
use crate::query::runner::run_mutation_one;
use crate::query::runner::ActorContext;
use crate::query::runner::CreateArgs;
use crate::query::runner::FilteredMutationKind;
use crate::query::runner::FindManyArgs;
use crate::query::runner::NestedRelationSelection;
use crate::query::runner::RunnerSchema;
use crate::state::AppState;

use super::schema::classify_root;
use super::schema::ResolverMethod;
use super::schema::RootClassification;
use super::schema::WorkspaceSchema;

pub const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(graphql_endpoint).get(graphql_get_endpoint))
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct GraphqlQueryParameters {
    #[serde(default)]
    query: Option<String>,
    #[serde(rename = "operationName", default)]
    operation_name: Option<String>,
    #[serde(default)]
    variables: Option<String>,
}

async fn graphql_get_endpoint(
    State(state): State<AppState>,
    Query(parameters): Query<GraphqlQueryParameters>,
    request: Request,
) -> Response {
    let variables = parameters
        .variables
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(Value::Null);

    execute_graphql(
        state,
        request,
        GraphqlRequestBody {
            query: parameters.query,
            operation_name: parameters.operation_name,
            variables: Some(variables),
            extensions: None,
        },
        RequestBodyKind::Json,
    )
    .await
}

async fn graphql_endpoint(State(state): State<AppState>, request: Request) -> Response {
    execute_graphql(
        state,
        request,
        GraphqlRequestBody {
            query: None,
            operation_name: None,
            variables: None,
            extensions: None,
        },
        RequestBodyKind::Http,
    )
    .await
}

enum RequestBodyKind {
    Http,
    Json,
}

async fn execute_graphql(
    state: AppState,
    request: Request,
    mut query_parameters: GraphqlRequestBody,
    body_kind: RequestBodyKind,
) -> Response {
    let (mut parts, body) = request.into_parts();
    let auth = match AuthContext::from_request_parts(&mut parts, &state).await {
        Ok(context) => context,
        Err(error) => return error_response(&parts.headers, &[auth_error(&error)]),
    };

    if auth.agent_id.is_some() {
        return error_response(
            &parts.headers,
            &[
                GraphQlError::forbidden("This token is only accepted by MCP")
                    .with_sub_code("INVALID_AUTH_CONTEXT"),
            ],
        );
    }

    let Some(workspace_id) = auth.workspace_id else {
        return error_response(
            &parts.headers,
            &[GraphQlError::forbidden("Could not retrieve workspace ID")
                .with_sub_code("WORKSPACE_NOT_DEFINED")],
        );
    };

    let body_kind_name = match body_kind {
        RequestBodyKind::Http => "http",
        RequestBodyKind::Json => "json",
    };
    let _ = body_kind_name;
    let content_type = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();

    if let RequestBodyKind::Http = body_kind {
        let bytes = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return error_response(
                    &parts.headers,
                    &[GraphQlError::validation(
                        "Request body was too large or could not be read.",
                    )],
                )
            }
        };

        if content_type == "application/graphql" {
            query_parameters.query = Some(String::from_utf8_lossy(&bytes).to_string());
        } else {
            let parsed: Result<GraphqlRequestBody, _> = serde_json::from_slice(&bytes);

            match parsed {
                Ok(parsed) => query_parameters = parsed,
                Err(_) => {
                    return error_response(
                        &parts.headers,
                        &[GraphQlError::validation(
                            "POST body missing, invalid, or not JSON.",
                        )],
                    )
                }
            }
        }
    }

    let execution_request = match parse_request(query_parameters) {
        Ok(request) => request,
        Err(error) => return error_response(&parts.headers, &[error]),
    };

    let maps = match load_flat_maps(&state, workspace_id).await {
        Ok(maps) => maps,
        Err(error) => return error_response(&parts.headers, &[error]),
    };
    let schema = WorkspaceSchema::build(&maps);

    if let Some(document) = execution_request.document.as_ref() {
        let fields = document
            .operations
            .iter()
            .flat_map(|operation| operation.fields.iter())
            .collect::<Vec<_>>();
        let has_core = fields
            .iter()
            .any(|field| classify_root(&field.name, &schema) == RootClassification::Core);
        let has_workspace = fields
            .iter()
            .any(|field| classify_root(&field.name, &schema) == RootClassification::Workspace);

        if has_core && has_workspace {
            return error_response(
                &parts.headers,
                &[GraphQlError::user_input(
                    "This query cannot be executed as a single request. Please split it into separate queries.",
                )
                .with_sub_code("CORE_WORKSPACE_MIX")],
            );
        }
    }

    let selected = match select_operation(&execution_request) {
        Ok(selected) => selected,
        Err(error) => return error_response(&parts.headers, &[error]),
    };

    let classification = classify_root(&selected.field.name, &schema);

    match classification {
        RootClassification::Introspection => {
            let value = introspection_value(&selected.field, &schema);
            data_response(&parts.headers, selected.field.response_key(), value)
        }
        RootClassification::Workspace => {
            match execute_workspace(
                &state,
                workspace_id,
                &schema,
                &maps,
                &auth,
                &selected.field,
                &execution_request,
                selected.kind == OperationKind::Query,
            )
            .await
            {
                Ok(value) => data_response(&parts.headers, selected.field.response_key(), value),
                Err(error) => error_response_for_key(
                    &parts.headers,
                    Some(selected.field.response_key()),
                    &[error],
                ),
            }
        }
        RootClassification::Core => {
            if has_workspace_field(&execution_request, &schema) {
                return error_response(
                    &parts.headers,
                    &[GraphQlError::user_input(
                        "This query cannot be executed as a single request. Please split it into separate queries.",
                    )
                    .with_sub_code("CORE_WORKSPACE_MIX")],
                );
            }

            match core_operation(&state, workspace_id, &auth, &selected.field).await {
                Ok(value) => data_response(&parts.headers, selected.field.response_key(), value),
                Err(error) => error_response_for_key(
                    &parts.headers,
                    Some(selected.field.response_key()),
                    &[error],
                ),
            }
        }
    }
}

fn has_workspace_field(request: &ExecutionRequest, schema: &WorkspaceSchema) -> bool {
    request
        .document
        .as_ref()
        .map(|document| {
            document
                .operations
                .iter()
                .flat_map(|operation| operation.fields.iter())
                .any(|field| classify_root(&field.name, schema) == RootClassification::Workspace)
        })
        .unwrap_or(false)
}

fn auth_error(error: &AuthError) -> GraphQlError {
    let message = error
        .messages
        .first()
        .cloned()
        .unwrap_or_else(|| "Authentication required".to_string());
    let mut mapped = match error.code {
        "UNAUTHENTICATED" | "USER_NOT_FOUND" | "WORKSPACE_NOT_FOUND" => {
            GraphQlError::unauthenticated(message)
        }
        "FORBIDDEN_EXCEPTION" | "INVALID_JWT_TOKEN_TYPE" | "USER_ALREADY_EXISTS" => {
            GraphQlError::forbidden(message)
        }
        "INVALID_INPUT" => GraphQlError::user_input(message),
        _ => GraphQlError::internal(message),
    };

    mapped = mapped.with_sub_code(error.code);
    mapped
}

fn error_response(_headers: &HeaderMap, errors: &[GraphQlError]) -> Response {
    error_response_for_key(_headers, None, errors)
}

fn error_response_for_key(
    _headers: &HeaderMap,
    root_key: Option<&str>,
    errors: &[GraphQlError],
) -> Response {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        Json(json!({
            "errors": errors.iter().map(GraphQlError::to_json).collect::<Vec<_>>(),
            "data": root_key.map(|key| json!({ key: Value::Null })).unwrap_or(Value::Null),
        })),
    )
        .into_response()
}

fn runner_graphql_error(error: crate::query::runner::RunnerError) -> GraphQlError {
    let message = error.message().to_string();
    let code = error.code();

    if code == "RECORD_NOT_FOUND" {
        return friendly_graphql_error(
            GraphQlError::not_found("Record not found"),
            code,
            "This record does not exist or has been deleted.",
        );
    }

    if matches!(
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
            | "UNSUPPORTED_OPERATOR"
            | "UNKNOWN_COLUMN"
            | "UNKNOWN_RELATION"
            | "UNKNOWN_SUB_FIELD"
            | "INVALID_DIRECTION"
    ) {
        return friendly_graphql_error(
            GraphQlError::user_input(message),
            code,
            "An error occurred.",
        );
    }

    if code == "UNKNOWN_OBJECT" {
        return friendly_graphql_error(
            GraphQlError::user_input(message),
            code,
            "An error occurred.",
        );
    }

    GraphQlError::internal(message).with_sub_code(code)
}

fn friendly_graphql_error(
    mut error: GraphQlError,
    sub_code: &str,
    user_friendly_message: &str,
) -> GraphQlError {
    if let Some(object) = error.extensions.as_object_mut() {
        object.insert(
            String::from("userFriendlyMessage"),
            Value::String(user_friendly_message.to_string()),
        );
    }

    error.with_sub_code(sub_code)
}

fn record_not_found_graphql_error() -> GraphQlError {
    friendly_graphql_error(
        GraphQlError::not_found("Record not found"),
        "RECORD_NOT_FOUND",
        "This record does not exist or has been deleted.",
    )
}

fn data_response(_headers: &HeaderMap, key: &str, value: Value) -> Response {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        Json(json!({ "data": { key: value } })),
    )
        .into_response()
}

#[allow(clippy::too_many_arguments)]
async fn execute_workspace(
    state: &AppState,
    workspace_id: Uuid,
    schema: &WorkspaceSchema,
    maps: &crate::metadata::flat::FlatWorkspaceMaps,
    auth: &AuthContext,
    field: &FieldSelection,
    request: &ExecutionRequest,
    is_query: bool,
) -> Result<Value, GraphQlError> {
    let _ = request;
    let resolver = schema.resolver(&field.name).ok_or_else(|| {
        GraphQlError::user_input(format!("Unknown workspace field {}", field.name))
    })?;
    let runner_schema = RunnerSchema::build(
        &workspace_id,
        schema.objects(),
        &maps.field_metadata_maps,
        &maps.index_metadata_maps,
    )
    .map_err(runner_graphql_error)?;
    let actor = if auth.workspace_member_id.is_some() {
        Some(ActorContext::manual(
            auth.workspace_member_id.map(|id| id.to_string()),
            None,
        ))
    } else {
        None
    };

    if is_query != resolver.method.is_query() {
        return Err(GraphQlError::user_input(
            "Operation kind does not match the workspace resolver.",
        ));
    }

    match resolver.method {
        ResolverMethod::FindMany => {
            find_many_root(state, schema, &runner_schema, field, resolver).await
        }
        ResolverMethod::FindOne => {
            find_one_root(state, schema, &runner_schema, field, resolver).await
        }
        ResolverMethod::CreateOne => {
            let data = required_object(field, "data")?;
            let upsert = optional_bool_arg(field, "upsert");
            let id = create_one(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                data,
                upsert,
                true,
                actor.as_ref(),
            )
            .await
            .map_err(runner_graphql_error)?;

            find_record(state, schema, &runner_schema, field, resolver, id, false).await
        }
        ResolverMethod::CreateMany => {
            let data = field
                .argument("data")
                .and_then(Value::as_array)
                .cloned()
                .ok_or_else(|| GraphQlError::user_input("CreateMany requires data"))?;
            let records = data
                .into_iter()
                .filter_map(|value| value.as_object().cloned())
                .collect::<Vec<_>>();
            let ids = create_many(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                &CreateArgs {
                    records,
                    upsert: optional_bool_arg(field, "upsert"),
                    backfill_position_undefined: true,
                    actor: actor.clone(),
                },
            )
            .await
            .map_err(runner_graphql_error)?;
            let filters = ids.iter().map(|id| id_filter(*id)).collect::<Vec<_>>();

            find_many(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                &FindManyArgs {
                    filter: Some(Value::Object(single_entry("or", Value::Array(filters)))),
                    selected_field_names: root_record_fields(schema, field, resolver),
                    ..Default::default()
                },
                false,
                &nested_relations(schema, field, resolver),
            )
            .await
            .map(|result| Value::Array(result.records))
            .map_err(runner_graphql_error)
        }
        ResolverMethod::UpdateOne => {
            mutate_existing_record(
                state,
                schema,
                &runner_schema,
                field,
                resolver,
                FilteredMutationKind::Update,
                field.argument("data").cloned(),
                false,
            )
            .await
        }
        ResolverMethod::DeleteOne => {
            let id = required_id(field)?;

            run_mutation_one(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                FilteredMutationKind::SoftDelete,
                Some(id_filter_value(id)),
                None,
                actor.as_ref(),
                false,
                vec![String::from("id")],
            )
            .await
            .map_err(runner_graphql_error)?;

            find_record(state, schema, &runner_schema, field, resolver, id, true).await
        }
        ResolverMethod::DestroyOne => {
            let id = required_id(field)?;
            let before =
                find_record(state, schema, &runner_schema, field, resolver, id, true).await?;

            run_mutation_one(
                &state.core_pool,
                &runner_schema,
                &resolver.object_name_singular,
                FilteredMutationKind::Destroy,
                Some(id_filter_value(id)),
                None,
                actor.as_ref(),
                true,
                vec![String::from("id")],
            )
            .await
            .map_err(runner_graphql_error)?;

            Ok(before)
        }
        ResolverMethod::RestoreOne => {
            mutate_existing_record(
                state,
                schema,
                &runner_schema,
                field,
                resolver,
                FilteredMutationKind::Restore,
                None,
                true,
            )
            .await
        }
        ResolverMethod::GroupBy => Err(GraphQlError::user_input(
            "GroupBy is executed by the group-by runner endpoint in a following slice",
        )),
    }
}

async fn find_many_root(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Result<Value, GraphQlError> {
    let mut args = FindManyArgs {
        filter: field.argument("filter").cloned(),
        order_by: field.argument("orderBy").cloned(),
        first: optional_usize(field, "first"),
        last: optional_usize(field, "last"),
        after: optional_string_arg(field, "after"),
        before: optional_string_arg(field, "before"),
        offset: field.argument("offset").and_then(Value::as_i64),
        selected_field_names: root_record_fields(schema, field, resolver),
        ..Default::default()
    };

    if args.first.is_none() && args.last.is_none() {
        args.first = Some(crate::query::runner::DEFAULT_PAGE_SIZE);
    }

    let result = find_many(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        &args,
        true,
        &nested_relations(schema, field, resolver),
    )
    .await
    .map_err(runner_graphql_error)?;
    let object_type = pascal_case(&resolver.object_name_singular);
    let edges = result
        .records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let id = record.get("id").and_then(Value::as_str).unwrap_or_default();
            let cursor = result.cursors.get(id).cloned().unwrap_or_else(|| {
                encode_cursor_data(&Map::from_iter([(
                    String::from("id"),
                    Value::String(id.to_string()),
                )]))
            });

            json!({
                "node": record,
                "cursor": cursor,
                "_index": index,
            })
        })
        .collect::<Vec<_>>();
    let connection = json!({
        "edges": edges,
        "pageInfo": {
            "startCursor": result.page_info.start_cursor,
            "endCursor": result.page_info.end_cursor,
            "hasNextPage": result.page_info.has_next_page,
            "hasPreviousPage": result.page_info.has_previous_page,
        },
        "totalCount": result.total_count,
        "__typename": format!("{object_type}Connection"),
    });

    Ok(project_connection(
        &connection,
        &field.selection,
        &object_type,
    ))
}

fn project_connection(
    connection: &Value,
    selections: &[FieldSelection],
    object_type: &str,
) -> Value {
    if selections.is_empty() {
        return connection.clone();
    }

    let mut output = Map::new();

    for selection in selections {
        let key = selection.response_key().to_string();

        match selection.name.as_str() {
            "edges" => {
                let nodes = connection
                    .get("edges")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let edges = nodes
                    .iter()
                    .map(|edge| {
                        let mut projected = Map::new();

                        for edge_selection in &selection.selection {
                            match edge_selection.name.as_str() {
                                "node" => {
                                    let node_type = format!("{object_type}WithRelations");
                                    projected.insert(
                                        edge_selection.response_key().to_string(),
                                        project_record(
                                            edge.get("node").unwrap_or(&Value::Null),
                                            &edge_selection.selection,
                                            &node_type,
                                        ),
                                    );
                                }
                                "cursor" => {
                                    projected.insert(
                                        edge_selection.response_key().to_string(),
                                        edge.get("cursor").cloned().unwrap_or(Value::Null),
                                    );
                                }
                                "__typename" => {
                                    projected.insert(
                                        edge_selection.response_key().to_string(),
                                        Value::String(format!("{object_type}Edge")),
                                    );
                                }
                                _ => {}
                            }
                        }

                        Value::Object(projected)
                    })
                    .collect::<Vec<_>>();

                output.insert(key, Value::Array(edges));
            }
            "pageInfo" => {
                output.insert(
                    key,
                    project_plain(connection.get("pageInfo"), &selection.selection, "PageInfo"),
                );
            }
            "totalCount" => {
                output.insert(
                    key,
                    connection.get("totalCount").cloned().unwrap_or(Value::Null),
                );
            }
            "__typename" => {
                output.insert(key, Value::String(format!("{object_type}Connection")));
            }
            _ => {}
        }
    }

    Value::Object(output)
}

fn project_plain(value: Option<&Value>, selections: &[FieldSelection], type_name: &str) -> Value {
    let value = value.cloned().unwrap_or(Value::Null);

    if selections.is_empty() {
        return value;
    }

    let mut output = Map::new();

    for selection in selections {
        output.insert(
            selection.response_key().to_string(),
            if selection.name == "__typename" {
                Value::String(type_name.to_string())
            } else {
                value.get(&selection.name).cloned().unwrap_or(Value::Null)
            },
        );
    }

    Value::Object(output)
}

fn project_record(record: &Value, selections: &[FieldSelection], type_name: &str) -> Value {
    let mut output = Map::new();

    for selection in selections {
        if selection.name == "__typename" {
            output.insert(
                selection.response_key().to_string(),
                Value::String(type_name.to_string()),
            );
            continue;
        }

        let child = record.get(&selection.name).cloned().unwrap_or(Value::Null);

        output.insert(
            selection.response_key().to_string(),
            if child.is_object() {
                project_record(&child, &selection.selection, selection.name.as_str())
            } else if child.is_array() {
                Value::Array(
                    child
                        .as_array()
                        .expect("array checked")
                        .iter()
                        .map(|item| {
                            project_record(item, &selection.selection, selection.name.as_str())
                        })
                        .collect(),
                )
            } else {
                child
            },
        );
    }

    Value::Object(output)
}

async fn find_one_root(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Result<Value, GraphQlError> {
    let filter = field
        .argument("filter")
        .cloned()
        .ok_or_else(|| GraphQlError::user_input("Missing filter argument"))?;
    let record = find_one(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        &FindOneArgs {
            filter: Some(filter),
            selected_field_names: root_record_fields_schemaless(field),
            ..Default::default()
        },
        &nested_relations(schema, field, resolver),
    )
    .await
    .map_err(runner_graphql_error)?
    .ok_or_else(record_not_found_graphql_error)?;

    Ok(project_record(
        &record,
        &field.selection,
        &pascal_case(&resolver.object_name_singular),
    ))
}

#[allow(clippy::too_many_arguments)]
async fn mutate_existing_record(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
    kind: FilteredMutationKind,
    data: Option<Value>,
    with_deleted: bool,
) -> Result<Value, GraphQlError> {
    let id = required_id(field)?;
    let actor = None;
    let selected = root_record_fields(schema, field, resolver);

    run_mutation_one(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        kind,
        Some(id_filter_value(id)),
        data,
        actor,
        with_deleted,
        selected.clone(),
    )
    .await
    .map_err(runner_graphql_error)?;

    find_record(
        state,
        schema,
        runner_schema,
        field,
        resolver,
        id,
        with_deleted,
    )
    .await
}

async fn find_record(
    state: &AppState,
    schema: &WorkspaceSchema,
    runner_schema: &RunnerSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
    id: Uuid,
    with_deleted: bool,
) -> Result<Value, GraphQlError> {
    let result = find_many(
        &state.core_pool,
        runner_schema,
        &resolver.object_name_singular,
        &FindManyArgs {
            filter: Some(id_filter_value(id)),
            with_deleted,
            selected_field_names: root_record_fields_schemaless(field),
            ..Default::default()
        },
        false,
        &nested_relations(schema, field, resolver),
    )
    .await
    .map_err(runner_graphql_error)?;
    let record = result
        .records
        .into_iter()
        .next()
        .ok_or_else(record_not_found_graphql_error)?;

    Ok(project_record(
        &record,
        &field.selection,
        &pascal_case(&resolver.object_name_singular),
    ))
}

fn root_record_fields(
    schema: &WorkspaceSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Vec<String> {
    let record_selection = node_selection(field);
    let mut names = root_record_fields_schemaless(field);

    if schema
        .fields(&resolver.object_name_singular)
        .iter()
        .any(|field| field.field_type == FieldMetadataType::Relation)
    {
        names.push(String::from("id"));
    }

    let _ = record_selection;
    names
}

fn root_record_fields_schemaless(field: &FieldSelection) -> Vec<String> {
    let record_selection = node_selection(field);
    let mut names = record_selection
        .iter()
        .filter(|selection| selection.name != "__typename")
        .map(|selection| selection.name.clone())
        .collect::<Vec<_>>();

    if !names.iter().any(|name| name == "id") {
        names.insert(0, String::from("id"));
    }

    names
}

fn nested_relations(
    schema: &WorkspaceSchema,
    field: &FieldSelection,
    resolver: &super::schema::RootResolver,
) -> Vec<NestedRelationSelection> {
    let record_selection = node_selection(field);
    let mut relations = Vec::new();

    for selection in record_selection {
        let Some(relation) = schema
            .fields(&resolver.object_name_singular)
            .iter()
            .find(|metadata| metadata.name == selection.name)
        else {
            continue;
        };

        if !matches!(
            relation.field_type,
            FieldMetadataType::Relation | FieldMetadataType::MorphRelation
        ) {
            continue;
        }

        let mut selected = selection
            .selection
            .iter()
            .map(|nested| nested.name.clone())
            .filter(|name| name != "__typename")
            .collect::<Vec<_>>();

        if !selected.iter().any(|name| name == "id") {
            selected.insert(0, String::from("id"));
        }

        relations.push(NestedRelationSelection {
            relation_field_name: selection.name.clone(),
            selected_field_names: selected,
            first: selection
                .argument("first")
                .and_then(Value::as_u64)
                .map(|value| value as usize),
        });
    }

    relations
}

fn node_selection(field: &FieldSelection) -> &[FieldSelection] {
    field
        .selection
        .iter()
        .find(|selection| selection.name == "edges")
        .and_then(|edges| {
            edges
                .selection
                .iter()
                .find(|selection| selection.name == "node")
        })
        .map(|node| node.selection.as_slice())
        .unwrap_or(field.selection.as_slice())
}

fn required_object(field: &FieldSelection, name: &str) -> Result<Map<String, Value>, GraphQlError> {
    field
        .argument(name)
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| GraphQlError::user_input(format!("{name} is required")))
}

fn required_id(field: &FieldSelection) -> Result<Uuid, GraphQlError> {
    field
        .argument("id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| GraphQlError::user_input("id is required"))
}

fn optional_bool_arg(field: &FieldSelection, name: &str) -> bool {
    field
        .argument(name)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn optional_string_arg(field: &FieldSelection, name: &str) -> Option<String> {
    field
        .argument(name)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn optional_usize(field: &FieldSelection, name: &str) -> Option<usize> {
    field
        .argument(name)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
}

fn id_filter(id: Uuid) -> Value {
    id_filter_value(id)
}

fn id_filter_value(id: Uuid) -> Value {
    Value::Object(single_entry(
        "id",
        Value::Object(single_entry("eq", Value::String(id.to_string()))),
    ))
}

fn single_entry(key: &str, value: Value) -> Map<String, Value> {
    Map::from_iter([(key.to_string(), value)])
}

async fn core_operation(
    state: &AppState,
    _workspace_id: Uuid,
    auth: &AuthContext,
    field: &FieldSelection,
) -> Result<Value, GraphQlError> {
    match field.name.as_str() {
        "currentUser" => current_user(state, auth, &field.selection).await,
        "currentWorkspace" => current_workspace(state, auth, &field.selection).await,
        "__typename" => Ok(Value::String(String::from("Query"))),
        _ => Err(GraphQlError::user_input(format!(
            "Core field \"{}\" is not implemented by this endpoint",
            field.name
        ))),
    }
}

async fn current_user(
    state: &AppState,
    auth: &AuthContext,
    selection: &[FieldSelection],
) -> Result<Value, GraphQlError> {
    let user_id = auth.user_id.ok_or_else(|| {
        GraphQlError::unauthenticated("Authentication required").with_sub_code("UNAUTHENTICATED")
    })?;
    let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, bool)>(
        "SELECT id, email, \"firstName\", \"lastName\", \"emailVerified\" \
         FROM core.\"user\" WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| GraphQlError::internal(error.to_string()))?
    .ok_or_else(|| GraphQlError::not_found("User was not found").with_sub_code("USER_NOT_FOUND"))?;
    let record = json!({
        "id": row.0,
        "email": row.1,
        "firstName": row.2,
        "lastName": row.3,
        "emailVerified": row.4,
        "__typename": "User",
    });

    Ok(project_record(&record, selection, "User"))
}

async fn current_workspace(
    state: &AppState,
    auth: &AuthContext,
    selection: &[FieldSelection],
) -> Result<Value, GraphQlError> {
    let workspace_id = auth.workspace_id.ok_or_else(|| {
        GraphQlError::unauthenticated("Authentication required").with_sub_code("UNAUTHENTICATED")
    })?;
    let row = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id, \"displayName\", subdomain FROM core.\"workspace\" WHERE id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| GraphQlError::internal(error.to_string()))?
    .ok_or_else(|| {
        GraphQlError::not_found("Workspace was not found").with_sub_code("WORKSPACE_NOT_FOUND")
    })?;
    let record = json!({
        "id": row.0,
        "displayName": row.1,
        "subdomain": row.2,
        "__typename": "Workspace",
    });

    Ok(project_record(&record, selection, "Workspace"))
}

fn schema_type(kind: &str, name: &str) -> Value {
    json!({ "kind": kind, "name": name })
}

fn named_type(name: &str) -> Value {
    schema_type("OBJECT", name)
}

fn non_null(inner: Value) -> Value {
    json!({
        "kind": "NON_NULL",
        "name": Value::Null,
        "ofType": inner,
    })
}

fn list(inner: Value) -> Value {
    json!({
        "kind": "LIST",
        "name": Value::Null,
        "ofType": inner,
    })
}

fn scalar_type(name: &str) -> Value {
    schema_type("SCALAR", name)
}

fn field_entry(name: &str, type_value: Value) -> Value {
    json!({
        "name": name,
        "type": type_value,
        "args": [],
    })
}

fn schema_type_specs(schema: &WorkspaceSchema) -> Vec<Value> {
    let mut specs = vec![
        field_entry("queryType", non_null(named_type("Query"))),
        field_entry("mutationType", named_type("Mutation")),
        field_entry("subscriptionType", Value::Null),
        field_entry("types", list(non_null(named_type("__Type")))),
        field_entry("directives", list(non_null(named_type("__Directive")))),
    ];

    specs.push(json!({
        "kind": "OBJECT",
        "name": "PageInfo",
        "fields": [
            field_entry("startCursor", scalar_type("String")),
            field_entry("endCursor", scalar_type("String")),
            field_entry("hasNextPage", non_null(scalar_type("Boolean"))),
            field_entry("hasPreviousPage", non_null(scalar_type("Boolean"))),
        ],
    }));

    let mut query_fields = Vec::new();
    let mut mutation_fields = Vec::new();

    for object in schema.objects() {
        let singular = camel_case(&object.name_singular);
        let plural = camel_case(&object.name_plural);
        let type_name = pascal_case(&object.name_singular);
        let connection_name = format!("{type_name}Connection");

        query_fields.push(field_entry(&plural, non_null(named_type(&connection_name))));
        query_fields.push(field_entry(&singular, non_null(named_type(&type_name))));

        let singular_pascal = pascal_case(&object.name_singular);
        let plural_pascal = pascal_case(&object.name_plural);

        for prefix in ["create", "update", "delete", "destroy", "restore"] {
            mutation_fields.push(field_entry(
                &format!("{prefix}{singular_pascal}"),
                non_null(named_type(&type_name)),
            ));
        }

        mutation_fields.push(field_entry(
            &format!("create{plural_pascal}"),
            non_null(list(non_null(named_type(&type_name)))),
        ));
    }

    specs.push(json!({
        "kind": "OBJECT",
        "name": "Query",
        "fields": query_fields,
    }));
    specs.push(json!({
        "kind": "OBJECT",
        "name": "Mutation",
        "fields": mutation_fields,
    }));

    for object in schema.objects() {
        let type_name = pascal_case(&object.name_singular);
        let mut fields = vec![field_entry("id", non_null(scalar_type("ID")))];

        for field in schema.fields(&object.name_singular) {
            let output_type = match field.field_type {
                FieldMetadataType::Relation => {
                    let relation_type = field
                        .settings
                        .as_ref()
                        .and_then(|settings| settings.get("relationType"))
                        .and_then(Value::as_str);

                    if relation_type == Some("ONE_TO_MANY") {
                        let target = relation_target(schema, field);
                        list(non_null(named_type(&format!("{target}Connection"))))
                    } else {
                        named_type(&relation_target(schema, field))
                    }
                }
                FieldMetadataType::Uuid => non_null(scalar_type("ID")),
                FieldMetadataType::Boolean => non_null(scalar_type("Boolean")),
                FieldMetadataType::Number
                | FieldMetadataType::Numeric
                | FieldMetadataType::Rating
                | FieldMetadataType::Position => scalar_type("Float"),
                FieldMetadataType::Array | FieldMetadataType::MultiSelect => {
                    list(non_null(scalar_type("String")))
                }
                _ => scalar_type(graphql_scalar_name(field.field_type)),
            };

            fields.push(field_entry(&field.name, output_type));
        }

        specs.push(json!({
            "kind": "OBJECT",
            "name": type_name,
            "fields": fields,
        }));

        let edge_name = format!("{type_name}Edge");
        let connection_name = format!("{type_name}Connection");
        specs.push(json!({
            "kind": "OBJECT",
            "name": edge_name,
            "fields": [
                field_entry("node", non_null(named_type(&type_name))),
                field_entry("cursor", non_null(scalar_type("String"))),
            ],
        }));
        specs.push(json!({
            "kind": "OBJECT",
            "name": connection_name,
            "fields": [
                field_entry("edges", list(non_null(named_type(&edge_name)))),
                field_entry("pageInfo", non_null(named_type("PageInfo"))),
                field_entry("totalCount", scalar_type("Int")),
            ],
        }));
    }

    specs
}

fn relation_target(
    schema: &WorkspaceSchema,
    field: &crate::metadata::flat::FlatFieldMetadata,
) -> String {
    field
        .relation_target_object_metadata_id
        .and_then(|target_id| {
            schema
                .objects()
                .iter()
                .find(|object| object.id == target_id)
        })
        .map(|object| pascal_case(&object.name_singular))
        .unwrap_or_else(|| String::from("JSON"))
}

fn graphql_scalar_name(field_type: FieldMetadataType) -> &'static str {
    match field_type {
        FieldMetadataType::Uuid => "ID",
        FieldMetadataType::Boolean => "Boolean",
        FieldMetadataType::Number
        | FieldMetadataType::Numeric
        | FieldMetadataType::Rating
        | FieldMetadataType::Position => "Float",
        FieldMetadataType::Date => "Date",
        FieldMetadataType::DateTime => "DateTime",
        FieldMetadataType::RawJson | FieldMetadataType::Files => "RawJson",
        _ => "String",
    }
}

fn introspection_value(field: &FieldSelection, schema: &WorkspaceSchema) -> Value {
    match field.name.as_str() {
        "__schema" => introspect_schema(&field.selection, schema),
        "__type" => {
            let name = field
                .argument("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            introspect_type(
                schema_type_specs(schema)
                    .into_iter()
                    .find(|spec| spec.get("name").and_then(Value::as_str) == Some(name))
                    .as_ref(),
                &field.selection,
            )
        }
        _ => Value::Null,
    }
}

fn introspect_schema(selections: &[FieldSelection], schema: &WorkspaceSchema) -> Value {
    let specs = schema_type_specs(schema);
    let mut output = Map::new();

    for selection in selections {
        let value = match selection.name.as_str() {
            "queryType" => introspect_type(
                specs
                    .iter()
                    .find(|spec| spec.get("name").and_then(Value::as_str) == Some("Query")),
                &selection.selection,
            ),
            "mutationType" => introspect_type(
                specs
                    .iter()
                    .find(|spec| spec.get("name").and_then(Value::as_str) == Some("Mutation")),
                &selection.selection,
            ),
            "subscriptionType" => Value::Null,
            "types" => Value::Array(
                specs
                    .iter()
                    .map(|spec| introspect_type(Some(spec), &selection.selection))
                    .collect(),
            ),
            "directives" => Value::Array(vec![]),
            _ => Value::Null,
        };

        output.insert(selection.response_key().to_string(), value);
    }

    Value::Object(output)
}

fn introspect_type(spec: Option<&Value>, selections: &[FieldSelection]) -> Value {
    let Some(spec) = spec else {
        return Value::Null;
    };

    if selections.is_empty() {
        return Value::Object(
            spec.get("name")
                .and_then(Value::as_str)
                .map(|name| {
                    Map::from_iter([(String::from("name"), Value::String(name.to_string()))])
                })
                .unwrap_or_default(),
        );
    }

    let mut output = Map::new();

    for selection in selections {
        let key = selection.response_key().to_string();
        let value = match selection.name.as_str() {
            "kind" => spec.get("kind").cloned().unwrap_or(Value::Null),
            "name" => spec.get("name").cloned().unwrap_or(Value::Null),
            "description" => Value::Null,
            "fields" => spec
                .get("fields")
                .and_then(Value::as_array)
                .map(|fields| {
                    fields
                        .iter()
                        .map(|field| introspect_field(field, &selection.selection))
                        .collect::<Vec<_>>()
                })
                .map(Value::Array)
                .unwrap_or(Value::Null),
            "args" => Value::Array(vec![]),
            "interfaces" => Value::Array(vec![]),
            "possibleTypes" => Value::Null,
            "enumValues" => Value::Null,
            "inputFields" => Value::Null,
            "ofType" => spec
                .get("ofType")
                .map(|inner| introspect_type(Some(inner), &selection.selection))
                .unwrap_or(Value::Null),
            _ => Value::Null,
        };

        output.insert(key, value);
    }

    Value::Object(output)
}

fn introspect_field(field: &Value, selections: &[FieldSelection]) -> Value {
    let mut output = Map::new();

    for selection in selections {
        let key = selection.response_key().to_string();
        let value = match selection.name.as_str() {
            "name" => field.get("name").cloned().unwrap_or(Value::Null),
            "description" => Value::Null,
            "type" => field
                .get("type")
                .map(|type_value| introspect_type(Some(type_value), &selection.selection))
                .unwrap_or(Value::Null),
            "args" => Value::Array(vec![]),
            "isDeprecated" => Value::Bool(false),
            "deprecationReason" => Value::Null,
            _ => Value::Null,
        };

        output.insert(key, value);
    }

    Value::Object(output)
}

fn unused_schema_helpers(schema: &WorkspaceSchema) -> HashMap<String, Vec<String>> {
    HashMap::from_iter([(String::from("query"), schema.query_root_fields())])
}

fn unused_camel_marker(value: &str) -> String {
    camel_case(value)
}
