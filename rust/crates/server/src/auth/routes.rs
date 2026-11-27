use axum::extract::ConnectInfo;
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use std::net::SocketAddr;
use uuid::Uuid;

use super::error::AuthResult;
use super::middleware::AuthContext;
use super::service::{self, CredentialsInput, LoginTokenInput, SignUpInput};
use super::session::{self, CreationOrigin, SessionRequestContext};
use crate::permissions::user_workspace_permissions_for_auth;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/check-user", post(check_user))
        .route("/auth/sign-up", post(sign_up))
        .route("/auth/login-token", post(login_token))
        .route("/auth/tokens", post(auth_tokens))
        .route("/auth/token", post(renew_token))
        .route("/auth/sign-out", post(sign_out))
        .route("/auth/me", get(me))
        .route("/auth/roles", get(roles))
        .route("/auth/api-keys", post(create_api_key))
        .route("/auth/api-keys/token", post(api_key_token))
        .route("/auth/api-keys/revoke", post(revoke_api_key))
        .merge(super::two_factor::router())
        .merge(super::account_tokens::router())
        .merge(super::workspace_lifecycle::router())
        .merge(super::sso::router())
}

fn response_with_cookies(
    status: StatusCode,
    body: serde_json::Value,
    cookies: Vec<HeaderValue>,
) -> Response {
    let mut response = (status, Json(body)).into_response();

    for cookie in cookies {
        response.headers_mut().append(SET_COOKIE, cookie);
    }

    response
}

fn session_context(headers: &HeaderMap, addr: Option<SocketAddr>) -> SessionRequestContext<'_> {
    SessionRequestContext {
        headers,
        ip_address: addr.map(|addr| addr.ip().to_string()),
    }
}

async fn check_user(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> AuthResult<(StatusCode, Json<serde_json::Value>)> {
    let email = body
        .get("email")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| super::error::AuthError::invalid_input("email is required"))?
        .to_string();

    let exists = if service::normalize_email(&email).is_err() {
        false
    } else {
        service::check_user_exists(&state, &email).await?
    };

    Ok((StatusCode::OK, Json(json!({ "exists": exists }))))
}

async fn sign_up(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<SignUpInput>,
) -> AuthResult<Response> {
    let signed_up = service::sign_up(&state, input).await?;

    let context = session_context(&headers, Some(addr));
    let cookies = session::issue_session_for_token_pair(
        &state,
        &context,
        &signed_up.tokens,
        CreationOrigin::SignIn,
    )
    .await?;

    Ok(response_with_cookies(
        StatusCode::CREATED,
        json!({
            "user": { "id": signed_up.user_id },
            "workspace": { "id": signed_up.workspace_id },
            "workspaceId": signed_up.workspace_id,
            "accessToken": signed_up.tokens.access_token,
            "refreshToken": signed_up.tokens.refresh_token,
        }),
        cookies,
    ))
}

async fn login_token(
    State(state): State<AppState>,
    Json(input): Json<CredentialsInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let login_token = service::get_login_token_from_credentials(&state, input).await?;

    Ok(Json(json!({ "loginToken": login_token })))
}

async fn auth_tokens(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<LoginTokenInput>,
) -> AuthResult<Response> {
    let (issued, email, user_id) = service::get_auth_tokens_from_login_token(&state, input).await?;

    let context = session_context(&headers, Some(addr));
    let cookies =
        session::issue_session_for_token_pair(&state, &context, &issued, CreationOrigin::SignIn)
            .await?;

    Ok(response_with_cookies(
        StatusCode::OK,
        json!({
            "accessToken": issued.access_token,
            "refreshToken": issued.refresh_token,
            "user": { "id": user_id, "email": email },
        }),
        cookies,
    ))
}

async fn renew_token(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<service::RenewTokenInput>,
) -> AuthResult<Response> {
    let issued = service::renew_token(&state, &input.refresh_token).await?;

    let context = session_context(&headers, Some(addr));
    let cookies = session::issue_session_for_token_pair(
        &state,
        &context,
        &issued,
        CreationOrigin::RenewalBridge,
    )
    .await?;

    Ok(response_with_cookies(
        StatusCode::OK,
        json!({
            "accessToken": issued.access_token,
            "refreshToken": issued.refresh_token,
        }),
        cookies,
    ))
}

async fn sign_out(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<service::SignOutInput>>,
) -> AuthResult<Response> {
    let session_token = session::extract_session_token(&headers, &state.config);

    let refresh_token = body.and_then(|Json(body)| body.refresh_token);

    session::sign_out(&state, session_token.as_deref(), refresh_token.as_deref()).await?;

    let mut cookies = Vec::new();

    if session::has_session_cookie(&headers) {
        cookies = session::session_cookie_clear_values(&state.config);
    }

    Ok(response_with_cookies(
        StatusCode::OK,
        json!({ "success": true }),
        cookies,
    ))
}

async fn roles(
    State(state): State<AppState>,
    context: AuthContext,
) -> AuthResult<Json<serde_json::Value>> {
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let roles = super::api_keys::roles_json(&state, workspace_id).await?;

    Ok(Json(roles))
}

async fn create_api_key(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<super::api_keys::CreateApiKeyInput>,
) -> AuthResult<(StatusCode, Json<serde_json::Value>)> {
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let created = super::api_keys::create_api_key(&state, workspace_id, input).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "apiKey": {
                "id": created.id,
                "name": created.name,
                "expiresAt": created.expires_at.to_rfc3339(),
            },
        })),
    ))
}

async fn api_key_token(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<super::api_keys::ApiKeyTokenInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let token = super::api_keys::generate_api_key_token(
        &state,
        workspace_id,
        input.api_key_id,
        match &input.expires_at {
            Some(raw) => Some(
                chrono::DateTime::parse_from_rfc3339(raw)
                    .map_err(|_| {
                        super::error::AuthError::invalid_input("expiresAt must be an ISO 8601 date")
                    })?
                    .with_timezone(&chrono::Utc),
            ),
            None => None,
        },
    )
    .await?;

    Ok(Json(json!({ "token": token })))
}

async fn revoke_api_key(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<super::api_keys::RevokeApiKeyInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let revoked = super::api_keys::revoke_api_key(&state, workspace_id, input.id).await?;

    Ok(Json(json!({
        "apiKey": super::api_keys::api_key_json(&revoked),
    })))
}

async fn me(
    State(state): State<AppState>,
    context: AuthContext,
) -> AuthResult<Json<serde_json::Value>> {
    let user_id = context
        .user_id
        .ok_or_else(|| super::error::AuthError::unauthenticated("Token invalid."))?;

    let user: Option<(Uuid, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, email::text, \"firstName\", \"lastName\" \
         FROM core.\"user\" WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (id, email, first_name, last_name) =
        user.ok_or_else(|| super::error::AuthError::user_not_found("User not found"))?;

    let workspace = match context.workspace_id {
        Some(workspace_id) => {
            let row: Option<(Uuid, Option<String>, String)> = sqlx::query_as(
                "SELECT id, \"displayName\", subdomain \
                 FROM core.\"workspace\" WHERE id = $1 AND \"deletedAt\" IS NULL",
            )
            .bind(workspace_id)
            .fetch_optional(&state.core_pool)
            .await?;

            row.map(|(id, display_name, subdomain)| {
                json!({
                    "id": id,
                    "displayName": display_name,
                    "subdomain": subdomain,
                })
            })
        }
        None => None,
    };

    let permissions = user_workspace_permissions_for_auth(&state, &context)
        .await
        .map_err(|error| super::error::AuthError::internal(error.message))?;

    Ok(Json(json!({
        "user": {
            "id": id,
            "email": email,
            "firstName": first_name.unwrap_or_default(),
            "lastName": last_name.unwrap_or_default(),
        },
        "workspace": workspace,
        "permissions": permissions,
        "authContext": {
            "tokenType": format!("{:?}", context.token_type),
            "userWorkspaceId": context.user_workspace_id,
            "workspaceMemberId": context.workspace_member_id,
            "apiKeyId": context.api_key_id,
            "authProvider": context.auth_provider,
            "isImpersonating": context.is_impersonating,
        },
    })))
}
