use nudge_core::ids::workspace_schema_name;
use serde::Deserialize;
use uuid::Uuid;

use super::crypto;
use super::error::{AuthError, AuthResult};
use super::jwt_service::TokenType;
use super::tokens::{self, AuthProvider, IssuedTokenPair};
use crate::state::AppState;

pub const ADMIN_ROLE_UNIVERSAL_IDENTIFIER: &str = "20202020-02c2-43f2-b94d-cab1f2b532eb";
pub const MEMBER_ROLE_UNIVERSAL_IDENTIFIER: &str = "20202020-a8a6-47ae-be79-e3601fa46a8e";

fn parse_universal_identifier(raw: &str) -> AuthResult<Uuid> {
    Uuid::parse_str(raw).map_err(|error| AuthError::internal(error.to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignUpInput {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub workspace_display_name: Option<String>,
    #[serde(default)]
    pub subdomain: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsInput {
    pub email: String,
    pub password: String,
    pub workspace_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginTokenInput {
    pub login_token: String,
    pub workspace_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenewTokenInput {
    pub refresh_token: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SignOutInput {
    #[serde(default)]
    pub refresh_token: Option<String>,
}

pub struct SignedUp {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub tokens: IssuedTokenPair,
}

pub fn normalize_email(email: &str) -> AuthResult<String> {
    let trimmed = email.trim().to_lowercase();

    let valid = !trimmed.is_empty()
        && trimmed.contains('@')
        && !trimmed.starts_with('@')
        && !trimmed.ends_with('@')
        && !trimmed.contains(char::is_whitespace);

    if valid {
        Ok(trimmed)
    } else {
        Err(AuthError::invalid_input(
            "Email is not a valid email address",
        ))
    }
}

pub async fn check_user_exists(state: &AppState, email: &str) -> AuthResult<bool> {
    let normalized = normalize_email(email)?;

    let row: Option<(bool,)> =
        sqlx::query_as("SELECT \"deletedAt\" IS NULL FROM core.\"user\" WHERE email = $1")
            .bind(&normalized)
            .fetch_optional(&state.core_pool)
            .await?;

    Ok(row.map(|(exists,)| exists).unwrap_or(false))
}

pub async fn sign_up(state: &AppState, input: SignUpInput) -> AuthResult<SignedUp> {
    let email = normalize_email(&input.email)?;

    if !crypto::is_valid_password(&input.password) {
        return Err(AuthError::invalid_input(
            "Password must be between 8 and 50 characters",
        ));
    }

    if check_user_exists(state, &email).await? {
        return Err(AuthError::user_already_exists(
            "There is already an account with this email",
        ));
    }

    let password_hash = crypto::hash_password(&input.password)
        .map_err(|error| AuthError::internal(error.to_string()))?;

    let locale = input.locale.unwrap_or_else(|| "en".to_string());

    let workspace_id = Uuid::new_v4();
    let application_id = Uuid::new_v4();
    let admin_role_id = Uuid::new_v4();
    let member_role_id = Uuid::new_v4();

    let schema_name = workspace_schema_name(&workspace_id);
    let subdomain = input
        .subdomain
        .map(|raw| raw.trim().to_lowercase())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| format!("ws-{}", schema_name.trim_start_matches("workspace_")));

    let display_name = input
        .workspace_display_name
        .filter(|raw| !raw.trim().is_empty())
        .unwrap_or_else(|| "My Workspace".to_string());

    let subdomain_taken: Option<(bool,)> =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM core.\"workspace\" WHERE subdomain = $1)")
            .bind(&subdomain)
            .fetch_optional(&state.core_pool)
            .await?;

    if subdomain_taken.map(|(taken,)| taken).unwrap_or(false) {
        return Err(AuthError::invalid_input("Subdomain is already taken"));
    }

    let mut transaction = state.core_pool.begin().await?;

    sqlx::query(
        "INSERT INTO core.\"workspace\" \
         (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
          \"workspaceCustomApplicationId\", \"isPasswordAuthEnabled\") \
         VALUES ($1, $2, $3, 'PENDING_CREATION', $4, $5, true)",
    )
    .bind(workspace_id)
    .bind(&display_name)
    .bind(Uuid::new_v4())
    .bind(&subdomain)
    .bind(application_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"application\" \
         (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", \"workspaceId\") \
         VALUES ($1, $2, 'Custom Workspace Application', 'LOCAL', 'INSTALLED', '', $3)",
    )
    .bind(application_id)
    .bind(Uuid::new_v4())
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"role\" \
         (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\", \
          \"canUpdateAllSettings\", \"canReadAllObjectRecords\", \"canCreateAllObjectRecords\", \
          \"canUpdateAllObjectRecords\", \"canSoftDeleteAllObjectRecords\", \
          \"canDestroyAllObjectRecords\") \
         VALUES ($1, $2, 'Admin', $3, $4, true, true, true, true, true, true)",
    )
    .bind(admin_role_id)
    .bind(parse_universal_identifier(ADMIN_ROLE_UNIVERSAL_IDENTIFIER)?)
    .bind(application_id)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"role\" \
         (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\") \
         VALUES ($1, $2, 'Member', $3, $4)",
    )
    .bind(member_role_id)
    .bind(parse_universal_identifier(
        MEMBER_ROLE_UNIVERSAL_IDENTIFIER,
    )?)
    .bind(application_id)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "UPDATE core.\"workspace\" \
         SET \"defaultRoleId\" = $1, \"databaseSchema\" = $2, \"activationStatus\" = 'CREATED', \
             \"updatedAt\" = now() \
         WHERE id = $3",
    )
    .bind(admin_role_id)
    .bind(&schema_name)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    let user_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO core.\"user\" (email, \"isEmailVerified\", \"passwordHash\", locale) \
         VALUES ($1, true, $2, $3) RETURNING id",
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(&locale)
    .fetch_one(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"userWorkspace\" (\"userId\", \"workspaceId\", locale) \
         VALUES ($1, $2, $3)",
    )
    .bind(user_id.0)
    .bind(workspace_id)
    .bind(&locale)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name))
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    let issued =
        tokens::issue_token_pair(state, user_id.0, workspace_id, AuthProvider::Password).await?;

    Ok(SignedUp {
        user_id: user_id.0,
        workspace_id,
        tokens: issued,
    })
}

async fn assert_password_auth_allowed(state: &AppState, workspace_id: &Uuid) -> AuthResult<()> {
    let row: Option<(bool, bool)> = sqlx::query_as(
        "SELECT \"isPasswordAuthEnabled\", \"isPasswordAuthBypassEnabled\" \
         FROM core.\"workspace\" WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (enabled, bypass) =
        row.ok_or_else(|| AuthError::workspace_not_found("Workspace not found"))?;

    if !enabled {
        return Err(AuthError::forbidden(
            "Password auth is not enabled on this workspace",
        ));
    }

    let _ = bypass;

    Ok(())
}

pub async fn get_login_token_from_credentials(
    state: &AppState,
    input: CredentialsInput,
) -> AuthResult<String> {
    let email = normalize_email(&input.email)?;

    assert_password_auth_allowed(state, &input.workspace_id).await?;

    let row: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, \"passwordHash\" FROM core.\"user\" \
         WHERE email = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(&email)
    .fetch_optional(&state.core_pool)
    .await?;

    let (user_id, password_hash) =
        row.ok_or_else(|| AuthError::user_not_found("User not found"))?;

    let member_row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(input.workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    if member_row.is_none() {
        return Err(AuthError::internal("User workspace not found"));
    }

    let hash = password_hash
        .filter(|hash| !hash.is_empty())
        .ok_or_else(|| AuthError::unauthenticated("Incorrect login method"))?;

    let matches = crypto::verify_password(&input.password, &hash)
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if !matches {
        return Err(AuthError::unauthenticated("Wrong password."));
    }

    tokens::issue_login_token(state, &email, input.workspace_id, AuthProvider::Password).await
}

pub async fn get_auth_tokens_from_login_token(
    state: &AppState,
    input: LoginTokenInput,
) -> AuthResult<(IssuedTokenPair, String, String)> {
    let payload = state
        .jwt
        .verify_typed(&input.login_token, TokenType::Login)
        .await?;

    let token_workspace = payload
        .get("workspaceId")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok());

    if token_workspace != Some(input.workspace_id) {
        return Err(AuthError::workspace_not_found("Workspace not found"));
    }

    let jwt_id = payload
        .get("jti")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
        .to_string();

    let email = payload
        .get("sub")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
        .to_string();

    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.\"user\" WHERE email = $1 AND \"deletedAt\" IS NULL")
            .bind(&email)
            .fetch_optional(&state.core_pool)
            .await?;

    let (user_id,) = row.ok_or_else(|| AuthError::user_not_found("User not found"))?;

    let member_row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(input.workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let member_row = member_row.ok_or_else(|| AuthError::internal("User workspace not found"))?;

    let two_factor: Option<(i64,)> = sqlx::query_as(
        "SELECT count(*) FROM core.\"twoFactorAuthenticationMethod\" \
         WHERE \"userWorkspaceId\" = $1 AND status = 'VERIFIED' AND \"deletedAt\" IS NULL",
    )
    .bind(member_row.0)
    .fetch_optional(&state.core_pool)
    .await?;

    if two_factor.map(|(count,)| count).unwrap_or(0) > 0 {
        return Err(AuthError::new(
            401,
            "TWO_FACTOR_AUTHENTICATION_VERIFICATION_REQUIRED",
            "Two factor authentication verification required",
        ));
    }

    let enforced: Option<(bool,)> = sqlx::query_as(
        "SELECT \"isTwoFactorAuthenticationEnforced\" FROM core.\"workspace\" WHERE id = $1",
    )
    .bind(input.workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    if enforced.map(|(value,)| value).unwrap_or(false) {
        return Err(AuthError::new(
            401,
            "TWO_FACTOR_AUTHENTICATION_PROVISION_REQUIRED",
            "Two factor authentication setup required",
        ));
    }

    let consumed_key = format!("login-token:consumed:{jwt_id}");

    let mut connection = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    let acquired: Option<String> = redis::cmd("SET")
        .arg(&consumed_key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(state.config.login_token_expires_in_secs)
        .query_async(&mut connection)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if acquired.is_none() {
        return Err(AuthError::unauthenticated(
            "Login token has already been used",
        ));
    }

    let issued =
        tokens::issue_token_pair(state, user_id, input.workspace_id, AuthProvider::Password)
            .await?;

    Ok((issued, email, user_id.to_string()))
}

pub async fn renew_token(state: &AppState, refresh_token: &str) -> AuthResult<IssuedTokenPair> {
    let verified = tokens::verify_refresh_token(state, refresh_token).await?;

    let grace = state.config.refresh_token_reuse_grace_period_secs as i64;

    if !verified.is_renewable(grace) {
        return Err(AuthError::unauthenticated("Token invalid."));
    }

    let mut transaction = state.core_pool.begin().await?;

    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended($1, 0))")
        .bind(format!("user-authentication:{}", verified.user_id))
        .execute(&mut *transaction)
        .await?;

    let fresh: Option<(Option<chrono::DateTime<chrono::Utc>>, Option<String>)> = sqlx::query_as(
        "SELECT \"revokedAt\", context->>'revokedReason' \
         FROM core.\"appToken\" \
         WHERE id = $1 AND type = 'REFRESH_TOKEN' AND \"deletedAt\" IS NULL",
    )
    .bind(verified.jwt_id)
    .fetch_optional(&mut *transaction)
    .await?;

    let (revoked_at, revoked_reason) =
        fresh.ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    let still_valid = revoked_reason.is_none()
        && revoked_at
            .map(|at| chrono::Utc::now() - at < chrono::Duration::seconds(grace))
            .unwrap_or(true);

    if !still_valid {
        return Err(AuthError::unauthenticated("Token invalid."));
    }

    let workspace_id = verified.workspace_id;

    sqlx::query(
        "UPDATE core.\"appToken\" \
         SET \"revokedAt\" = now(), \"updatedAt\" = now(), \
             context = COALESCE(context, '{}'::jsonb) || '{\"issuedByRotation\":true}'::jsonb \
         WHERE id = $1",
    )
    .bind(verified.jwt_id)
    .execute(&mut *transaction)
    .await?;

    let new_token_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO core.\"appToken\" (\"userId\", \"workspaceId\", type, \"expiresAt\") \
         VALUES ($1, $2, 'REFRESH_TOKEN', now() + make_interval(secs => $3)) RETURNING id",
    )
    .bind(verified.user_id)
    .bind(workspace_id)
    .bind(state.config.refresh_token_expires_in_secs as i64)
    .fetch_one(&mut *transaction)
    .await?;

    let auth_provider = verified
        .payload
        .get("authProvider")
        .and_then(serde_json::Value::as_str)
        .and_then(AuthProvider::parse)
        .unwrap_or(AuthProvider::Password);

    let targeted = verified
        .payload
        .get("targetedTokenType")
        .and_then(serde_json::Value::as_str)
        .and_then(TokenType::parse)
        .unwrap_or(TokenType::Access);

    let access_or_agnostic = match (targeted, workspace_id) {
        (TokenType::Access, Some(workspace_id)) => {
            tokens::issue_access_token(state, verified.user_id, workspace_id, auth_provider).await?
        }
        _ => tokens::issue_workspace_agnostic_token(state, verified.user_id, auth_provider).await?,
    };

    let claims = match workspace_id {
        Some(id) => serde_json::json!({
            "sub": verified.user_id,
            "userId": verified.user_id,
            "workspaceId": id,
            "authProvider": auth_provider.as_str(),
            "targetedTokenType": targeted.as_str(),
            "isImpersonating": false,
        }),
        None => serde_json::json!({
            "sub": verified.user_id,
            "userId": verified.user_id,
            "authProvider": auth_provider.as_str(),
            "targetedTokenType": targeted.as_str(),
            "isImpersonating": false,
        }),
    };

    let new_refresh = state
        .jwt
        .sign(
            claims,
            TokenType::Refresh,
            state.config.refresh_token_expires_in_secs,
            Some(&new_token_id.0),
        )
        .await?;

    transaction.commit().await?;

    Ok(IssuedTokenPair {
        access_token: access_or_agnostic,
        refresh_token: new_refresh,
    })
}
