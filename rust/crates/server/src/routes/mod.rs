pub mod health;

use axum::extract::OriginalUri;
use axum::http::{Method, StatusCode};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::auth;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let auth_routes = auth::routes::router();

    Router::new()
        .route("/healthz", get(health::healthz))
        .merge(auth_routes)
        .nest("/metadata", crate::metadata::api::router())
        .nest("/graphql", crate::query::entry::graphql::router())
        .nest("/rest", crate::query::entry::rest::router())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::csrf::cookie_session_csrf,
        ))
        .fallback(not_found_fallback)
        .with_state(state)
}

async fn not_found_fallback(method: Method, OriginalUri(uri): OriginalUri) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "message": format!("Cannot {} {}", method, uri.path()),
            "error": "Not Found",
            "statusCode": 404,
        })),
    )
        .into_response()
}
