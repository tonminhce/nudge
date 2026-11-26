use serde_json::{json, Value};
use uuid::Uuid;

use super::error::{AuthError, AuthResult};
use super::jwt_service::TokenType;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthProvider {
    Google,
    Microsoft,
    Password,
    Sso,
    Impersonation,
}

impl AuthProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthProvider::Google => "google",
            AuthProvider::Microsoft => "microsoft",
            AuthProvider::Password => "password",
            AuthProvider::Sso => "sso",
            AuthProvider::Impersonation => "impersonation",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "google" => AuthProvider::Google,
            "microsoft" => AuthProvider::Microsoft,
            "password" => AuthProvider::Password,
            "sso" => AuthProvider::Sso,
            "impersonation" => AuthProvider::Impersonation,
            _ => return None,
        })
    }
}

pub struct IssuedTokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn user_workspace_id(
    state: &AppState,
    user_id: &Uuid,
    workspace_id: &Uuid,
) -> AuthResult<Uuid> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    row.map(|(id,)| id)
        .ok_or_else(|| AuthError::user_not_found("User not found in this workspace"))
}

pub async fn issue_access_token(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
    auth_provider: AuthProvider,
) -> AuthResult<String> {
    let user_workspace_id = user_workspace_id(state, &user_id, &workspace_id).await?;

    let claims = json!({
        "sub": user_id,
        "userId": user_id,
        "workspaceId": workspace_id,
        "userWorkspaceId": user_workspace_id,
        "authProvider": auth_provider.as_str(),
        "isImpersonating": false,
    });

    state
        .jwt
        .sign(
            claims,
            TokenType::Access,
            state.config.access_token_expires_in_secs,
            None,
        )
        .await
}

pub async fn issue_workspace_agnostic_token(
    state: &AppState,
    user_id: Uuid,
    auth_provider: AuthProvider,
) -> AuthResult<String> {
    let claims = json!({
        "sub": user_id,
        "userId": user_id,
        "authProvider": auth_provider.as_str(),
    });

    state
        .jwt
        .sign(
            claims,
            TokenType::WorkspaceAgnostic,
            state.config.workspace_agnostic_token_expires_in_secs,
            None,
        )
        .await
}

pub async fn issue_login_token(
    state: &AppState,
    email: &str,
    workspace_id: Uuid,
    auth_provider: AuthProvider,
) -> AuthResult<String> {
    let claims = json!({
        "sub": email,
        "workspaceId": workspace_id,
        "authProvider": auth_provider.as_str(),
    });

    let jwt_id = Uuid::new_v4();

    state
        .jwt
        .sign(
            claims,
            TokenType::Login,
            state.config.login_token_expires_in_secs,
            Some(&jwt_id),
        )
        .await
}

pub async fn issue_refresh_token(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Option<Uuid>,
    auth_provider: AuthProvider,
    targeted_token_type: TokenType,
) -> AuthResult<String> {
    let token_row: (Uuid,) = sqlx::query_as(
        "INSERT INTO core.\"appToken\" (\"userId\", \"workspaceId\", type, \"expiresAt\") \
         VALUES ($1, $2, 'REFRESH_TOKEN', now() + make_interval(secs => $3)) RETURNING id",
    )
    .bind(user_id)
    .bind(workspace_id)
    .bind(state.config.refresh_token_expires_in_secs as i64)
    .fetch_one(&state.core_pool)
    .await?;

    let claims = match workspace_id {
        Some(id) => json!({
            "sub": user_id,
            "userId": user_id,
            "workspaceId": id,
            "authProvider": auth_provider.as_str(),
            "targetedTokenType": targeted_token_type.as_str(),
            "isImpersonating": false,
        }),
        None => json!({
            "sub": user_id,
            "userId": user_id,
            "authProvider": auth_provider.as_str(),
            "targetedTokenType": targeted_token_type.as_str(),
            "isImpersonating": false,
        }),
    };

    state
        .jwt
        .sign(
            claims,
            TokenType::Refresh,
            state.config.refresh_token_expires_in_secs,
            Some(&token_row.0),
        )
        .await
}

pub async fn issue_token_pair(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
    auth_provider: AuthProvider,
) -> AuthResult<IssuedTokenPair> {
    let access_token = issue_access_token(state, user_id, workspace_id, auth_provider).await?;

    let refresh_token = issue_refresh_token(
        state,
        user_id,
        Some(workspace_id),
        auth_provider,
        TokenType::Access,
    )
    .await?;

    Ok(IssuedTokenPair {
        access_token,
        refresh_token,
    })
}

pub struct VerifiedRefreshToken {
    pub payload: Value,
    pub jwt_id: Uuid,
    pub user_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked_reason: Option<String>,
}

impl VerifiedRefreshToken {
    pub fn is_renewable(&self, grace_period_secs: i64) -> bool {
        match self.revoked_reason {
            Some(_) => false,
            None => match self.revoked_at {
                None => true,
                Some(revoked_at) => {
                    chrono::Utc::now() - revoked_at < chrono::Duration::seconds(grace_period_secs)
                }
            },
        }
    }
}

pub async fn verify_refresh_token(
    state: &AppState,
    refresh_token: &str,
) -> AuthResult<VerifiedRefreshToken> {
    let payload = state
        .jwt
        .verify_typed(refresh_token, TokenType::Refresh)
        .await?;

    let jwt_id = payload
        .get("jti")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    let user_id = payload
        .get("userId")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    let workspace_id = payload
        .get("workspaceId")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok());

    let row: Option<(Option<chrono::DateTime<chrono::Utc>>, Option<String>)> = sqlx::query_as(
        "SELECT \"revokedAt\", context->>'revokedReason' \
         FROM core.\"appToken\" \
         WHERE id = $1 AND type = 'REFRESH_TOKEN' AND \"deletedAt\" IS NULL",
    )
    .bind(jwt_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (revoked_at, revoked_reason) =
        row.ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    Ok(VerifiedRefreshToken {
        payload,
        jwt_id,
        user_id,
        workspace_id,
        revoked_at,
        revoked_reason,
    })
}
