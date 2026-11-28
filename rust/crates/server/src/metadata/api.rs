#![allow(dead_code)]

pub mod errors;
pub mod graphql;
pub mod handlers;
pub mod input;
pub mod service;

use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::extract::Query;
use axum::extract::Request;
use axum::extract::State;
use axum::http::header::ACCEPT;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::http::Method;
use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::response::sse::Sse;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde_json::json;
use serde_json::Value;
use tokio::time::MissedTickBehavior;

use crate::auth::error::AuthError;
use crate::auth::middleware::AuthContext;
use crate::events::EventStreamRegistry;
use crate::metadata::api::errors::GraphQlError;
use crate::metadata::api::graphql::parse_request;
use crate::metadata::api::graphql::select_operation;
use crate::metadata::api::graphql::FieldSelection;
use crate::metadata::api::graphql::GraphqlRequestBody;
use crate::metadata::api::graphql::OperationKind;
use crate::metadata::api::handlers::dispatch;
use crate::metadata::api::service::OperationContext;
use crate::state::AppState;

pub const GRAPHQL_RESPONSE_CONTENT_TYPE: &str = "application/graphql-response+json";
pub const JSON_CONTENT_TYPE: &str = "application/json";

const MAX_BODY_BYTES: usize = 1024 * 1024;

impl GraphQlError {
    pub fn from_auth_error(error: &AuthError) -> Self {
        let message = error
            .messages
            .first()
            .cloned()
            .unwrap_or_else(|| "Request failed".to_string());

        let mapped = match error.code {
            "UNAUTHENTICATED" => Self::unauthenticated(message),
            "FORBIDDEN_EXCEPTION" | "INVALID_JWT_TOKEN_TYPE" | "USER_ALREADY_EXISTS" => {
                Self::forbidden(message)
            }
            "USER_NOT_FOUND" | "WORKSPACE_NOT_FOUND" => Self::unauthenticated(message),
            "INVALID_INPUT" => Self::user_input(message),
            _ => Self::internal(message),
        };

        mapped.with_sub_code(error.code)
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(graphql_get_endpoint).post(graphql_endpoint))
        .fallback(method_not_allowed)
}

async fn method_not_allowed(method: Method) -> Response {
    graphql_json_response(
        StatusCode::METHOD_NOT_ALLOWED,
        &HeaderMap::new(),
        error_payload(&[GraphQlError::method_not_allowed(format!(
            "{method} not supported."
        ))]),
    )
}

async fn graphql_endpoint(State(state): State<AppState>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return graphql_json_response(
                StatusCode::OK,
                &parts.headers,
                error_payload(&[GraphQlError::validation(
                    "Request body was too large or could not be read.",
                )]),
            )
        }
    };
    let body: GraphqlRequestBody = match serde_json::from_slice(&bytes) {
        Ok(body) => body,
        Err(_) => {
            return graphql_json_response(
                StatusCode::OK,
                &parts.headers,
                error_payload(&[GraphQlError::validation(
                    "POST body missing, invalid, or not JSON.",
                )]),
            )
        }
    };
    execute_metadata_request(state, parts, body).await
}

async fn graphql_get_endpoint(
    State(state): State<AppState>,
    Query(parameters): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let Some(query) = parameters.get("query") else {
        return graphql_json_response(
            StatusCode::OK,
            &headers,
            json!({
                "errors": [GraphQlError::new("Must provide query string.", "BAD_REQUEST").to_json()]
            }),
        );
    };
    let variables = match parameters.get("variables") {
        Some(raw) => match serde_json::from_str(raw) {
            Ok(value) => Some(value),
            Err(_) => {
                return graphql_json_response(
                    StatusCode::OK,
                    &headers,
                    error_payload(&[GraphQlError::validation(
                        "POST body missing, invalid, or not JSON.",
                    )]),
                )
            }
        },
        None => None,
    };
    let (mut parts, _) = Request::builder().uri("/").body(()).unwrap().into_parts();
    parts.headers = headers;
    execute_metadata_request(
        state,
        parts,
        GraphqlRequestBody {
            query: Some(query.clone()),
            operation_name: parameters.get("operationName").cloned(),
            variables,
            extensions: None,
        },
    )
    .await
}

async fn execute_metadata_request(
    state: AppState,
    mut parts: axum::http::request::Parts,
    body: GraphqlRequestBody,
) -> Response {
    let execution_request = match parse_request(body) {
        Ok(execution_request) => execution_request,
        Err(error) => {
            return graphql_json_response(StatusCode::OK, &parts.headers, error_payload(&[error]))
        }
    };
    let selected = match select_operation(&execution_request) {
        Ok(selected) => selected,
        Err(error) => {
            return graphql_json_response(StatusCode::OK, &parts.headers, error_payload(&[error]))
        }
    };
    if selected.kind != OperationKind::Subscription && !is_metadata_root_field(&selected.field.name)
    {
        return graphql_json_response(
            StatusCode::OK,
            &parts.headers,
            json!({
                "errors": [{
                    "message": format!("Cannot query field \"{}\" on type \"Query\".", selected.field.name),
                    "locations": [{
                        "line": selected.field.line,
                        "column": selected.field.column,
                    }],
                    "extensions": { "code": "GRAPHQL_VALIDATION_FAILED" }
                }]
            }),
        );
    }
    if selected.kind == OperationKind::Query
        && handlers::is_public_metadata_root_field(&selected.field.name)
    {
        let payload =
            handlers::dispatch_public(&state, &selected.field, &execution_request.variables).await;
        return graphql_json_response(StatusCode::OK, &parts.headers, payload.to_json());
    }
    let auth = match AuthContext::from_request_parts(&mut parts, &state).await {
        Ok(context) => context,
        Err(error) => {
            return graphql_json_response(
                StatusCode::OK,
                &parts.headers,
                error_payload(&[GraphQlError::from_auth_error(&error)]),
            )
        }
    };
    let Some(workspace_id) = auth.workspace_id else {
        return graphql_json_response(
            StatusCode::OK,
            &parts.headers,
            error_payload(&[GraphQlError::forbidden("Could not retrieve workspace ID")
                .with_sub_code("WORKSPACE_NOT_DEFINED")]),
        );
    };
    if selected.kind == OperationKind::Subscription {
        if selected.field.name != "onEventSubscription" {
            return graphql_json_response(
                StatusCode::OK,
                &parts.headers,
                error_payload(&[GraphQlError::validation(format!(
                    "Cannot subscribe to field \"{}\".",
                    selected.field.name
                ))]),
            );
        }
        return event_subscription_response(
            &state,
            &auth,
            &selected.field,
            &execution_request.variables,
        )
        .await;
    }
    let ctx = OperationContext {
        state: &state,
        workspace_id,
        auth: Some(&auth),
        selection: &selected.field,
        variables: &execution_request.variables,
        is_query: selected.kind == OperationKind::Query,
    };
    let payload = dispatch(&ctx).await;
    graphql_json_response(StatusCode::OK, &parts.headers, payload.to_json())
}

fn is_metadata_root_field(name: &str) -> bool {
    matches!(
        name,
        "createOneObject"
            | "updateOneObject"
            | "updateManyObjects"
            | "deleteOneObject"
            | "createOneField"
            | "createFieldMetadata"
            | "updateOneField"
            | "updateFieldMetadata"
            | "deleteOneField"
            | "deleteFieldMetadata"
            | "createOneIndex"
            | "deleteOneIndex"
            | "createWebhook"
            | "updateWebhook"
            | "deleteWebhook"
            | "webhooks"
            | "webhook"
            | "objects"
            | "fields"
            | "object"
            | "field"
            | "currentUser"
            | handlers::PUBLIC_WORKSPACE_DATA_FIELD
            | "addQueryToEventStream"
            | "removeQueryFromEventStream"
    )
}

struct EventStreamCleanup {
    registry: EventStreamRegistry,
    workspace_id: uuid::Uuid,
    event_stream_id: String,
}

impl Drop for EventStreamCleanup {
    fn drop(&mut self) {
        self.registry
            .remove(self.workspace_id, &self.event_stream_id);
    }
}

async fn event_subscription_response(
    state: &AppState,
    auth: &AuthContext,
    field: &FieldSelection,
    variables: &Value,
) -> Response {
    let event_stream_id = field
        .argument("eventStreamId")
        .and_then(Value::as_str)
        .or_else(|| variables.get("eventStreamId").and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let Some(event_stream_id) = event_stream_id else {
        return graphql_json_response(
            StatusCode::OK,
            &axum::http::HeaderMap::new(),
            error_payload(&[GraphQlError::validation("eventStreamId is required")]),
        );
    };
    let Some(workspace_id) = auth.workspace_id else {
        return graphql_json_response(
            StatusCode::OK,
            &axum::http::HeaderMap::new(),
            error_payload(&[GraphQlError::forbidden("Could not retrieve workspace ID")]),
        );
    };
    let Some(mut receiver) = state
        .event_streams
        .open(workspace_id, event_stream_id.clone(), auth)
    else {
        return graphql_json_response(
            StatusCode::CONFLICT,
            &axum::http::HeaderMap::new(),
            error_payload(&[GraphQlError::new(
                "Event stream already exists",
                "EVENT_STREAM_ALREADY_EXISTS",
            )]),
        );
    };
    let heartbeat_registry = state.event_streams.clone();
    let cleanup = EventStreamCleanup {
        registry: state.event_streams.clone(),
        workspace_id,
        event_stream_id: event_stream_id.clone(),
    };
    let stream = async_stream::stream! {
        let _cleanup = cleanup;
        let initial = json!({
            "data": {
                "onEventSubscription": {
                    "eventStreamId": event_stream_id,
                    "objectRecordEventsWithQueryIds": [],
                    "metadataEvents": [],
                    "queueJobEvents": [],
                }
            }
        });
        yield Ok::<Event, Infallible>(Event::default().event("next").data(initial.to_string()));
        let mut interval = tokio::time::interval(EventStreamRegistry::keepalive_interval());
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        interval.tick().await;
        let mut last_ttl_refresh_at = 0_u64;
        loop {
            tokio::select! {
                payload = receiver.recv() => {
                    let Some(payload) = payload else {
                        break;
                    };
                    let data = json!({"data": {"onEventSubscription": payload}});
                    yield Ok::<Event, Infallible>(Event::default().event("next").data(data.to_string()));
                }
                _ = interval.tick() => {
                    if heartbeat_registry
                        .heartbeat(workspace_id, &event_stream_id, &mut last_ttl_refresh_at)
                        .is_none()
                    {
                        yield Ok::<Event, Infallible>(Event::default().event("next").data("{}"));
                    }
                }
            }
        }
    };
    Sse::new(stream).into_response()
}

fn error_payload(errors: &[GraphQlError]) -> Value {
    json!({
        "errors": errors.iter().map(GraphQlError::to_json).collect::<Vec<Value>>()
    })
}

fn graphql_json_response(status: StatusCode, headers: &HeaderMap, payload: Value) -> Response {
    (
        status,
        [(CONTENT_TYPE, negotiated_content_type(headers))],
        Json(payload),
    )
        .into_response()
}

fn negotiated_content_type(headers: &HeaderMap) -> &'static str {
    let accept = headers
        .get(ACCEPT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if accept.contains(GRAPHQL_RESPONSE_CONTENT_TYPE) {
        return GRAPHQL_RESPONSE_CONTENT_TYPE;
    }

    JSON_CONTENT_TYPE
}
