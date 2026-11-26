use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::error::{AuthError, AuthResult};
use super::jwt_service::TokenType;
use super::middleware::AuthContext;
use crate::state::AppState;

const API_KEY_MAX_TOKEN_LIFETIME_SECS: i64 = 100 * (365 * 24 * 60 * 60 + 6 * 60 * 60);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyInput {
    pub name: String,
    pub expires_at: String,
    #[serde(default)]
    pub revoked_at: Option<String>,
    pub role_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyTokenInput {
    pub api_key_id: Uuid,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeApiKeyInput {
    pub id: Uuid,
}

#[derive(Clone)]
pub struct ApiKeyRow {
    pub id: Uuid,
    pub name: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl ApiKeyRow {
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now > self.expires_at
    }
}

fn parse_iso_date(raw: &str, field: &str) -> AuthResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|_| AuthError::invalid_input(format!("{field} must be an ISO 8601 date")))
}

fn api_key_not_found(id: Uuid) -> AuthError {
    AuthError::new(
        404,
        "API_KEY_NOT_FOUND",
        format!("API Key with id {id} not found"),
    )
}

type ApiKeyColumns = (Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>);

async fn fetch_api_key(
    state: &AppState,
    id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<Option<ApiKeyRow>> {
    let row: Option<ApiKeyColumns> = sqlx::query_as(
        "SELECT id, name, \"expiresAt\", \"revokedAt\" \
         FROM core.\"apiKey\" WHERE id = $1 AND \"workspaceId\" = $2",
    )
    .bind(id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row.map(|(id, name, expires_at, revoked_at)| ApiKeyRow {
        id,
        name,
        expires_at,
        revoked_at,
    }))
}

pub async fn validate_api_key_for_auth(
    state: &AppState,
    workspace_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
) -> AuthResult<()> {
    let Some(workspace_id) = workspace_id else {
        return Err(AuthError::forbidden("This API Key is revoked"));
    };

    let workspace_exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.\"workspace\" WHERE id = $1 AND \"deletedAt\" IS NULL")
            .bind(workspace_id)
            .fetch_optional(&state.core_pool)
            .await?;

    if workspace_exists.is_none() {
        return Err(AuthError::workspace_not_found("Workspace not found"));
    }

    let Some(api_key_id) = api_key_id else {
        return Err(AuthError::forbidden("This API Key is revoked"));
    };

    let api_key = fetch_api_key(state, api_key_id, workspace_id)
        .await?
        .ok_or_else(|| AuthError::forbidden("This API Key is revoked"))?;

    if api_key.is_revoked() {
        return Err(AuthError::forbidden("This API Key is revoked"));
    }

    if api_key.is_expired(Utc::now()) {
        return Err(AuthError::forbidden("This API Key is expired"));
    }

    Ok(())
}

async fn validate_api_key_for_minting(
    state: &AppState,
    workspace_id: Uuid,
    api_key_id: Uuid,
) -> AuthResult<ApiKeyRow> {
    let api_key = fetch_api_key(state, api_key_id, workspace_id)
        .await?
        .ok_or_else(|| api_key_not_found(api_key_id))?;

    if api_key.is_revoked() {
        return Err(AuthError::new(
            403,
            "API_KEY_REVOKED",
            "This API Key is revoked",
        ));
    }

    if api_key.is_expired(Utc::now()) {
        return Err(AuthError::new(
            403,
            "API_KEY_EXPIRED",
            "This API Key has expired",
        ));
    }

    Ok(api_key)
}

pub fn token_lifetime_secs(expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> i64 {
    match expires_at {
        Some(expires_at) => (expires_at - now).num_seconds(),
        None => API_KEY_MAX_TOKEN_LIFETIME_SECS,
    }
}

pub async fn generate_api_key_token(
    state: &AppState,
    workspace_id: Uuid,
    api_key_id: Uuid,
    expires_at: Option<DateTime<Utc>>,
) -> AuthResult<String> {
    validate_api_key_for_minting(state, workspace_id, api_key_id).await?;

    let lifetime = token_lifetime_secs(expires_at, Utc::now()).max(0) as u64;

    let claims = json!({
        "sub": workspace_id,
        "workspaceId": workspace_id,
    });

    state
        .jwt
        .sign(claims, TokenType::ApiKey, lifetime, Some(&api_key_id))
        .await
}

pub struct CreatedApiKey {
    pub id: Uuid,
    pub name: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn create_api_key(
    state: &AppState,
    workspace_id: Uuid,
    input: CreateApiKeyInput,
) -> AuthResult<CreatedApiKey> {
    if input.name.trim().is_empty() {
        return Err(AuthError::invalid_input("Name must not be empty"));
    }

    let expires_at = parse_iso_date(&input.expires_at, "expiresAt")?;
    let revoked_at = match &input.revoked_at {
        Some(raw) => Some(parse_iso_date(raw, "revokedAt")?),
        None => None,
    };

    let role_exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.role WHERE id = $1 AND \"workspaceId\" = $2")
            .bind(input.role_id)
            .bind(workspace_id)
            .fetch_optional(&state.core_pool)
            .await?;

    if role_exists.is_none() {
        return Err(AuthError::invalid_input("Role not found in this workspace"));
    }

    let api_key_id = Uuid::new_v4();

    let mut transaction = state.core_pool.begin().await?;

    sqlx::query(
        "INSERT INTO core.\"apiKey\" (id, name, \"expiresAt\", \"revokedAt\", \"workspaceId\") \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(api_key_id)
    .bind(input.name.trim())
    .bind(expires_at)
    .bind(revoked_at)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"roleTarget\" \
         (\"workspaceId\", \"roleId\", \"apiKeyId\", \"universalIdentifier\", \"applicationId\") \
         SELECT $1, $2, $3, $4, w.\"workspaceCustomApplicationId\" \
         FROM core.\"workspace\" w WHERE w.id = $1",
    )
    .bind(workspace_id)
    .bind(input.role_id)
    .bind(api_key_id)
    .bind(Uuid::new_v4())
    .execute(&mut *transaction)
    .await?;

    let target_count: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.\"roleTarget\" \
         WHERE \"apiKeyId\" = $1 AND \"workspaceId\" = $2",
    )
    .bind(api_key_id)
    .bind(workspace_id)
    .fetch_one(&mut *transaction)
    .await?;

    if target_count.0 != 1 {
        return Err(AuthError::internal(
            "Failed to bind the role to the API key",
        ));
    }

    transaction.commit().await?;

    Ok(CreatedApiKey {
        id: api_key_id,
        name: input.name.trim().to_string(),
        expires_at,
    })
}

pub async fn revoke_api_key(
    state: &AppState,
    workspace_id: Uuid,
    id: Uuid,
) -> AuthResult<ApiKeyRow> {
    sqlx::query(
        "UPDATE core.\"apiKey\" SET \"revokedAt\" = now(), \"updatedAt\" = now() \
         WHERE id = $1 AND \"workspaceId\" = $2",
    )
    .bind(id)
    .bind(workspace_id)
    .execute(&state.core_pool)
    .await?;

    fetch_api_key(state, id, workspace_id)
        .await?
        .ok_or_else(|| api_key_not_found(id))
}

pub fn api_key_json(api_key: &ApiKeyRow) -> serde_json::Value {
    json!({
        "id": api_key.id,
        "name": api_key.name,
        "expiresAt": api_key.expires_at.to_rfc3339(),
        "revokedAt": api_key.revoked_at.map(|at| at.to_rfc3339()),
    })
}

pub async fn roles_json(state: &AppState, workspace_id: Uuid) -> AuthResult<serde_json::Value> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, label FROM core.role WHERE \"workspaceId\" = $1 \
         ORDER BY \"createdAt\" ASC, id ASC",
    )
    .bind(workspace_id)
    .fetch_all(&state.core_pool)
    .await?;

    let roles: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, label)| json!({ "id": id, "label": label }))
        .collect();

    Ok(json!({ "roles": roles }))
}

pub fn workspace_id_from(context: &AuthContext) -> AuthResult<Uuid> {
    context
        .workspace_id
        .ok_or_else(|| AuthError::forbidden("This endpoint requires a workspace-scoped token"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_lifetime_uses_the_expiry_or_the_100y_default() {
        let now = Utc::now();

        assert_eq!(
            token_lifetime_secs(None, now),
            API_KEY_MAX_TOKEN_LIFETIME_SECS
        );
        assert_eq!(
            token_lifetime_secs(Some(now + chrono::Duration::hours(1)), now),
            3600
        );
        assert_eq!(
            token_lifetime_secs(Some(now - chrono::Duration::hours(1)), now),
            -3600
        );
    }

    #[test]
    fn api_key_rows_track_revocation_and_expiry() {
        let now = Utc::now();

        let active = ApiKeyRow {
            id: Uuid::new_v4(),
            name: "k".to_string(),
            expires_at: now + chrono::Duration::days(1),
            revoked_at: None,
        };

        assert!(!active.is_revoked());
        assert!(!active.is_expired(now));

        let expired = ApiKeyRow {
            expires_at: now - chrono::Duration::seconds(1),
            ..active.clone()
        };

        assert!(expired.is_expired(now));

        let revoked = ApiKeyRow {
            revoked_at: Some(now),
            ..active.clone()
        };

        assert!(revoked.is_revoked());
    }

    #[test]
    fn iso_parsing_rejects_non_dates_with_the_field_name() {
        let error = parse_iso_date("not-a-date", "expiresAt").unwrap_err();

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.messages[0].contains("expiresAt"));
    }
}
