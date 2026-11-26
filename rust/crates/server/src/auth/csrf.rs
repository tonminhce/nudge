use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use super::session;
use crate::state::AppState;

const SAFE_METHODS: [Method; 3] = [Method::GET, Method::HEAD, Method::OPTIONS];

fn has_bearer_token(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| !token.is_empty())
}

pub async fn cookie_session_csrf(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    if SAFE_METHODS.contains(&method) {
        return next.run(request).await;
    }

    if has_bearer_token(&headers) {
        return next.run(request).await;
    }

    if session::extract_session_token(&headers, &state.config).is_none() {
        return next.run(request).await;
    }

    let origin_allowed = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .filter(|origin| !origin.is_empty())
        .is_some_and(|origin| session::is_request_origin_allowed(&headers, &state.config, origin));

    if origin_allowed {
        return next.run(request).await;
    }

    (
        axum::http::StatusCode::FORBIDDEN,
        Json(json!({
            "statusCode": 403,
            "messages": [session::CSRF_ORIGIN_MISMATCH_MESSAGE],
            "error": "CSRF_ORIGIN_MISMATCH",
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_presence_skips_the_check_but_empty_schemes_do_not() {
        let mut headers = HeaderMap::new();
        assert!(!has_bearer_token(&headers));

        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer abc".parse().unwrap(),
        );
        assert!(has_bearer_token(&headers));

        let mut bare = HeaderMap::new();
        bare.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer ".parse().unwrap(),
        );
        assert!(!has_bearer_token(&bare));

        let mut basic = HeaderMap::new();
        basic.insert(
            axum::http::header::AUTHORIZATION,
            "Basic abc".parse().unwrap(),
        );
        assert!(!has_bearer_token(&basic));
    }

    #[test]
    fn only_get_head_and_options_are_safe() {
        assert!(SAFE_METHODS.contains(&Method::GET));
        assert!(SAFE_METHODS.contains(&Method::HEAD));
        assert!(SAFE_METHODS.contains(&Method::OPTIONS));
        assert!(!SAFE_METHODS.contains(&Method::POST));
        assert!(!SAFE_METHODS.contains(&Method::DELETE));
    }
}
