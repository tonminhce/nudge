#![allow(dead_code)]

pub mod errors;
pub mod graphql;
pub mod handlers;
pub mod input;
pub mod service;

use axum::extract::FromRequestParts;
use axum::extract::Request;
use axum::extract::State;
use axum::http::header::ACCEPT;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::http::Method;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde_json::json;
use serde_json::Value;

use crate::auth::error::AuthError;
use crate::auth::middleware::AuthContext;
use crate::metadata::api::errors::GraphQlError;
use crate::metadata::api::graphql::parse_request;
use crate::metadata::api::graphql::select_operation;
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
        .route("/", post(graphql_endpoint))
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
    let (mut parts, body) = request.into_parts();

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

    let ctx = OperationContext {
        state: &state,
        workspace_id,
        selection: &selected.field,
        variables: &execution_request.variables,
        is_query: selected.kind == OperationKind::Query,
    };

    let payload = dispatch(&ctx).await;

    graphql_json_response(StatusCode::OK, &parts.headers, payload.to_json())
}

fn error_payload(errors: &[GraphQlError]) -> Value {
    json!({
        "errors": errors.iter().map(GraphQlError::to_json).collect::<Vec<Value>>(),
        "data": Value::Null
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
