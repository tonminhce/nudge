use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use serde_json::Value;
use uuid::Uuid;

use super::error::{AuthError, AuthResult};
use super::jwt_service::TokenType;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub token_type: TokenType,
    pub user_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub user_workspace_id: Option<Uuid>,
    pub workspace_member_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub auth_provider: Option<String>,
    pub is_impersonating: bool,
}

fn uuid_claim(payload: &Value, key: &str) -> Option<Uuid> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
}

fn string_claim(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(|raw| raw.to_string())
}

fn parse_payload(payload: &Value) -> AuthResult<AuthContext> {
    let token_type = string_claim(payload, "type")
        .and_then(|raw| TokenType::parse(&raw))
        .ok_or_else(|| AuthError::invalid_jwt_token_type("Unsupported token type"))?;

    match token_type {
        TokenType::ApiKey => {
            let workspace_id = uuid_claim(payload, "workspaceId")
                .or_else(|| uuid_claim(payload, "sub"))
                .ok_or_else(|| AuthError::invalid_jwt_token_type("Unsupported token type"))?;

            Ok(AuthContext {
                token_type,
                user_id: None,
                workspace_id: Some(workspace_id),
                user_workspace_id: None,
                workspace_member_id: None,
                api_key_id: uuid_claim(payload, "jti"),
                auth_provider: None,
                is_impersonating: false,
            })
        }
        TokenType::WorkspaceAgnostic => Ok(AuthContext {
            token_type,
            user_id: uuid_claim(payload, "userId").or_else(|| uuid_claim(payload, "sub")),
            workspace_id: None,
            user_workspace_id: None,
            workspace_member_id: None,
            api_key_id: None,
            auth_provider: string_claim(payload, "authProvider"),
            is_impersonating: false,
        }),
        TokenType::Access | TokenType::Playground => {
            let user_id = uuid_claim(payload, "userId")
                .or_else(|| uuid_claim(payload, "sub"))
                .ok_or_else(|| AuthError::invalid_jwt_token_type("Unsupported token type"))?;

            let workspace_id = uuid_claim(payload, "workspaceId")
                .ok_or_else(|| AuthError::invalid_jwt_token_type("Unsupported token type"))?;

            Ok(AuthContext {
                token_type,
                user_id: Some(user_id),
                workspace_id: Some(workspace_id),
                user_workspace_id: uuid_claim(payload, "userWorkspaceId"),
                workspace_member_id: uuid_claim(payload, "workspaceMemberId"),
                api_key_id: None,
                auth_provider: string_claim(payload, "authProvider"),
                is_impersonating: payload
                    .get("isImpersonating")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        }
        _ => Err(AuthError::invalid_jwt_token_type("Unsupported token type")),
    }
}

impl FromRequestParts<AppState> for AuthContext {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> AuthResult<Self> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AuthError::unauthenticated("Missing authorization header"))?;

        let token = header
            .strip_prefix("Bearer ")
            .filter(|token| !token.is_empty())
            .ok_or_else(|| {
                AuthError::unauthenticated("Authorization header must use the Bearer scheme")
            })?;

        let payload = state.jwt.verify(token).await?;

        parse_payload(&payload)
    }
}
