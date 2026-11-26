use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};
use serde_json::json;

use super::error::AuthResult;
use crate::state::AppState;

pub mod config;
pub mod flow;
pub mod google;
pub mod microsoft;
pub mod oidc;
pub mod saml;
pub mod state_store;

const UNRESERVED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

pub fn percent_encode(raw: &str) -> String {
    percent_encoding::utf8_percent_encode(raw, UNRESERVED).to_string()
}

pub fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn redirect_response(location: &str) -> Response {
    (
        axum::http::StatusCode::FOUND,
        [(axum::http::header::LOCATION, location.to_string())],
    )
        .into_response()
}

pub fn optional_query_string(raw: Option<&str>) -> Option<String> {
    raw.filter(|raw| !raw.is_empty()).map(String::from)
}

async fn exchange_token(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> AuthResult<Response> {
    let plain_token = body
        .get("ssoExchangeToken")
        .and_then(serde_json::Value::as_str)
        .filter(|raw| !raw.is_empty())
        .ok_or_else(|| super::error::AuthError::invalid_input("ssoExchangeToken is required"))?;

    let consumed = flow::consume_sso_exchange_token(&state, plain_token).await?;

    let workspace_agnostic = super::tokens::issue_workspace_agnostic_token(
        &state,
        consumed.user_id,
        consumed.auth_provider,
    )
    .await?;

    let refresh_token = super::tokens::issue_refresh_token(
        &state,
        consumed.user_id,
        None,
        consumed.auth_provider,
        super::jwt_service::TokenType::WorkspaceAgnostic,
    )
    .await?;

    let context = super::session::SessionRequestContext {
        headers: &headers,
        ip_address: Some(addr.ip().to_string()),
    };

    let token_pair = super::tokens::IssuedTokenPair {
        access_token: workspace_agnostic.clone(),
        refresh_token: refresh_token.clone(),
    };

    let cookies = super::session::issue_session_for_token_pair(
        &state,
        &context,
        &token_pair,
        super::session::CreationOrigin::SignIn,
    )
    .await?;

    let mut response = (
        axum::http::StatusCode::OK,
        Json(json!({
            "accessToken": workspace_agnostic,
            "refreshToken": refresh_token,
            "tokens": {
                "accessOrWorkspaceAgnosticToken": workspace_agnostic,
                "refreshToken": refresh_token,
            },
        })),
    )
        .into_response();

    for cookie in cookies {
        response
            .headers_mut()
            .append(axum::http::header::SET_COOKIE, cookie);
    }

    Ok(response)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/google", get(google::auth_start))
        .route("/auth/google/redirect", get(google::auth_redirect))
        .route("/auth/microsoft", get(microsoft::auth_start))
        .route("/auth/microsoft/redirect", get(microsoft::auth_redirect))
        .route("/auth/oidc/login/{identityProviderId}", get(oidc::login))
        .route("/auth/oidc/callback", get(oidc::callback))
        .route(
            "/auth/saml/metadata/{identityProviderId}",
            get(saml::metadata),
        )
        .route("/auth/saml/login/{identityProviderId}", get(saml::login))
        .route(
            "/auth/saml/callback/{identityProviderId}",
            post(saml::callback),
        )
        .route("/auth/sso/exchange-token", post(exchange_token))
}
