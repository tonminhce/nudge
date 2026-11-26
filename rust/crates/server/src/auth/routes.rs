use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use super::error::AuthResult;
use super::middleware::AuthContext;
use super::service::{self, CredentialsInput, LoginTokenInput, RenewTokenInput, SignUpInput};
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TokensResponse {
    access_token: String,
    refresh_token: String,
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

    let exists = service::check_user_exists(&state, &email).await?;

    Ok((StatusCode::OK, Json(json!({ "exists": exists }))))
}

async fn sign_up(
    State(state): State<AppState>,
    Json(input): Json<SignUpInput>,
) -> AuthResult<(StatusCode, Json<serde_json::Value>)> {
    let signed_up = service::sign_up(&state, input).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "user": { "id": signed_up.user_id },
            "workspace": { "id": signed_up.workspace_id },
            "workspaceId": signed_up.workspace_id,
            "accessToken": signed_up.tokens.access_token,
            "refreshToken": signed_up.tokens.refresh_token,
        })),
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
    Json(input): Json<LoginTokenInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let (issued, email, user_id) = service::get_auth_tokens_from_login_token(&state, input).await?;

    Ok(Json(json!({
        "accessToken": issued.access_token,
        "refreshToken": issued.refresh_token,
        "user": { "id": user_id, "email": email },
    })))
}

async fn renew_token(
    State(state): State<AppState>,
    Json(input): Json<RenewTokenInput>,
) -> AuthResult<Json<TokensResponse>> {
    let issued = service::renew_token(&state, &input.refresh_token).await?;

    Ok(Json(TokensResponse {
        access_token: issued.access_token,
        refresh_token: issued.refresh_token,
    }))
}

async fn sign_out(
    State(state): State<AppState>,
    Json(input): Json<RenewTokenInput>,
) -> AuthResult<(StatusCode, Json<serde_json::Value>)> {
    service::sign_out(&state, &input.refresh_token).await?;

    Ok((StatusCode::OK, Json(json!({ "success": true }))))
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

    Ok(Json(json!({
        "user": {
            "id": id,
            "email": email,
            "firstName": first_name.unwrap_or_default(),
            "lastName": last_name.unwrap_or_default(),
        },
        "workspace": workspace,
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
