pub mod health;

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use axum::body::Body;
use axum::extract::OriginalUri;
use axum::http::header::CACHE_CONTROL;
use axum::http::header::CONTENT_LENGTH;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderValue;
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
    if matches!(method, Method::GET | Method::HEAD) {
        if let Some(response) = frontend_response(uri.path(), method == Method::HEAD).await {
            return response;
        }
    }
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

async fn frontend_response(path: &str, head: bool) -> Option<Response> {
    let relative = safe_frontend_path(path)?;
    let directory = std::env::var("NUDGE_FRONTEND_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("crates/front-app/dist"));
    let file = directory.join(&relative);
    let (file, cache_control) = if file.is_file() {
        (file, "public, max-age=31536000, immutable")
    } else if relative.extension().is_none() {
        (directory.join("index.html"), "no-cache")
    } else {
        return None;
    };
    let content_type = frontend_content_type(&file);
    let file = tokio::fs::File::open(&file).await.ok()?;
    let size = file.metadata().await.ok()?.len();
    let body = if head {
        Body::empty()
    } else {
        Body::from_stream(tokio_util::io::ReaderStream::new(file))
    };
    Some(
        (
            StatusCode::OK,
            [
                (CONTENT_TYPE, HeaderValue::from_static(content_type)),
                (CACHE_CONTROL, HeaderValue::from_static(cache_control)),
                (CONTENT_LENGTH, HeaderValue::from(size)),
            ],
            body,
        )
            .into_response(),
    )
}

fn safe_frontend_path(path: &str) -> Option<PathBuf> {
    let relative = Path::new(path.trim_start_matches('/'));
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return None;
    }
    Some(relative.to_path_buf())
}

fn frontend_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("json") => "application/json",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_paths_reject_parent_traversal() {
        assert!(safe_frontend_path("/../Cargo.toml").is_none());
        assert!(safe_frontend_path("/assets/../../Cargo.toml").is_none());
        assert_eq!(
            safe_frontend_path("/assets/app.js"),
            Some(PathBuf::from("assets/app.js"))
        );
    }

    #[test]
    fn frontend_content_types_cover_trunk_assets() {
        assert_eq!(
            frontend_content_type(Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            frontend_content_type(Path::new("app.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            frontend_content_type(Path::new("app.wasm")),
            "application/wasm"
        );
    }
}
