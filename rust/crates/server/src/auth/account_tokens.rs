use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use redis::Script;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::error::{AuthError, AuthResult};
use super::jwt_service::TokenType;
use super::middleware::AuthContext;
use super::session;
use super::tokens::{self, AuthProvider};
use crate::state::AppState;

const EMAIL_VERIFICATION_TOKEN_EXPIRES_IN_SECS: i64 = 60 * 60;
const PASSWORD_RESET_TOKEN_EXPIRES_IN_SECS: i64 = 5 * 60;
const INVITATION_TOKEN_EXPIRES_IN_SECS: i64 = 30 * 24 * 60 * 60;

const EMAIL_VERIFICATION_RESEND_COOLDOWN_SECS: i64 = 60;
const VERIFICATION_EMAIL_SENDING_LIMIT: i64 = 5;
const VERIFICATION_EMAIL_SENDING_WINDOW_SECS: i64 = 86_400;

const PASSWORD_RESET_EMAIL_RATE_LIMIT_MAX: i64 = 3;
const PASSWORD_RESET_EMAIL_RATE_LIMIT_WINDOW_SECS: i64 = 15 * 60;

const INVITATION_SENDING_BY_EMAIL_LIMIT: i64 = 10;
const INVITATION_SENDING_BY_EMAIL_WINDOW_SECS: i64 = 7 * 24 * 60 * 60;
const INVITATION_SENDING_BY_WORKSPACE_LIMIT: i64 = 500;
const INVITATION_SENDING_BY_WORKSPACE_WINDOW_SECS: i64 = 7 * 24 * 60 * 60;
const INVITATION_SENDING_BY_SENDER_LIMIT: i64 = 20;
const INVITATION_SENDING_BY_SENDER_SAME_WORK_DOMAIN_LIMIT: i64 = 50;
const INVITATION_SENDING_BY_SENDER_WINDOW_SECS: i64 = 86_400;

const PASSWORD_RESET_EMAIL_THROTTLE_PREFIX: &str = "password-reset-email:";
const VERIFICATION_EMAIL_THROTTLE_PREFIX: &str = "verification-email-sending:throttler:";
const INVITATION_SENDING_THROTTLE_PREFIX: &str = "invitation-resending-workspace:throttler:";

const APP_TOKEN_TYPE_EMAIL_VERIFICATION: &str = "EMAIL_VERIFICATION_TOKEN";
const APP_TOKEN_TYPE_PASSWORD_RESET: &str = "PASSWORD_RESET_TOKEN";
const APP_TOKEN_TYPE_INVITATION: &str = "INVITATION_TOKEN";
const APP_TOKEN_TYPE_ONBOARDING_INVITATION: &str = "ONBOARDING_INVITATION_TOKEN";

const CONSUME_BUCKET_SCRIPT: &str = "local count = redis.call('INCRBY', KEYS[1], ARGV[1]) \
if redis.call('TTL', KEYS[1]) < 0 then redis.call('EXPIRE', KEYS[1], ARGV[2]) end \
return count";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/verify-email", post(verify_email_login_token))
        .route(
            "/auth/verify-email/workspace-agnostic-token",
            post(verify_email_workspace_agnostic_token),
        )
        .route(
            "/auth/resend-email-verification-token",
            post(resend_email_verification_token),
        )
        .route(
            "/auth/email-password-reset-link",
            post(email_password_reset_link),
        )
        .route(
            "/auth/validate-password-reset-token",
            post(validate_password_reset_token),
        )
        .route(
            "/auth/update-password-via-reset-token",
            post(update_password_via_reset_token),
        )
        .route(
            "/auth/workspace-invitations",
            post(send_invitations).get(find_workspace_invitations),
        )
        .route(
            "/auth/workspace-invitations/resend",
            post(resend_workspace_invitation),
        )
        .route(
            "/auth/workspace-invitations/delete",
            post(delete_workspace_invitation),
        )
        .route(
            "/auth/workspace-invitations/accept",
            post(accept_workspace_invitation),
        )
        .route(
            "/auth/workspace-invite-hash-valid",
            post(workspace_invite_hash_valid),
        )
        .route(
            "/auth/workspace-from-invite-hash",
            post(workspace_from_invite_hash),
        )
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn generate_plain_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);

    hex::encode(bytes)
}

fn token_expired(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now > expires_at
}

fn invitation_token_types() -> &'static [&'static str] {
    &[
        APP_TOKEN_TYPE_INVITATION,
        APP_TOKEN_TYPE_ONBOARDING_INVITATION,
    ]
}

fn invitation_emails_match(context_email: &str, presented_email: &str) -> bool {
    context_email.eq_ignore_ascii_case(presented_email)
}

fn bucket_admits(count: i64, limit: i64) -> bool {
    count <= limit
}

fn is_email_verification_required() -> bool {
    std::env::var("IS_EMAIL_VERIFICATION_REQUIRED")
        .map(|raw| raw.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn validate_invitation_emails(emails: &[String]) -> AuthResult<Vec<String>> {
    let mut normalized = Vec::with_capacity(emails.len());

    for email in emails {
        normalized.push(super::service::normalize_email(email)?);
    }

    for email in &normalized {
        if normalized.iter().filter(|other| *other == email).count() > 1 {
            return Err(AuthError::invalid_input(
                "Emails must be unique in an invitation batch",
            ));
        }
    }

    Ok(normalized)
}

fn reset_password_link(base_url: &str, token: &str) -> String {
    format!("{}/reset-password/{token}", base_url.trim_end_matches('/'))
}

fn verification_link(base_url: &str, token: &str, email: &str) -> String {
    format!(
        "{}/verify-email?emailVerificationToken={}&email={}",
        base_url.trim_end_matches('/'),
        token,
        email
    )
}

fn invitation_link(base_url: &str, token: &str, email: &str) -> String {
    format!(
        "{}/welcome?inviteToken={}&email={}",
        base_url.trim_end_matches('/'),
        token,
        email
    )
}

fn password_reset_subject(has_password: bool) -> &'static str {
    if has_password {
        "Action Needed to Reset Password"
    } else {
        "Action Needed to Set Password"
    }
}

fn subdomain_url(config: &nudge_core::Config, subdomain: Option<&str>) -> String {
    match subdomain {
        Some(subdomain) if !subdomain.is_empty() => {
            let (scheme, rest) = config
                .server_url
                .split_once("://")
                .unwrap_or(("http", config.server_url.as_str()));

            let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);

            format!("{scheme}://{subdomain}.{host}")
        }
        _ => config.server_url.clone(),
    }
}

fn cooldown_wait_message(remaining_secs: i64) -> String {
    let minutes = remaining_secs / 60;
    let seconds = remaining_secs % 60;

    if minutes > 0 {
        format!(
            "Please wait {minutes} minutes {seconds} seconds before requesting another verification email"
        )
    } else {
        format!("Please wait {seconds} seconds before requesting another verification email")
    }
}

fn limit_reached_error() -> AuthError {
    AuthError::new(
        429,
        "LIMIT_REACHED",
        "Rate limit reached. Please try again later.",
    )
}

async fn redis_connection(state: &AppState) -> AuthResult<redis::aio::MultiplexedConnection> {
    state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

async fn consume_bucket(
    state: &AppState,
    key: &str,
    amount: i64,
    window_secs: i64,
) -> AuthResult<i64> {
    let mut connection = redis_connection(state).await?;

    Script::new(CONSUME_BUCKET_SCRIPT)
        .key(key)
        .arg(amount)
        .arg(window_secs)
        .invoke_async(&mut connection)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

#[derive(Debug, sqlx::FromRow)]
struct EmailVerificationTokenRow {
    pub id: Uuid,
    #[sqlx(rename = "userId")]
    pub user_id: Uuid,
    #[sqlx(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
    pub context: Option<Value>,
}

#[derive(Debug, sqlx::FromRow)]
struct AppTokenRow {
    pub id: Uuid,
    #[sqlx(rename = "workspaceId")]
    pub workspace_id: Option<Uuid>,
    #[sqlx(rename = "expiresAt")]
    pub expires_at: DateTime<Utc>,
    pub value: String,
    pub r#type: String,
    pub context: Option<Value>,
}

fn invitation_email_from_context(context: Option<&Value>) -> Option<String> {
    context
        .and_then(|context| context.get("email"))
        .and_then(Value::as_str)
        .map(String::from)
}

fn invitation_role_id_from_context(context: Option<&Value>) -> Option<Uuid> {
    context
        .and_then(|context| context.get("roleId"))
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
}

fn invitation_json(row: &AppTokenRow) -> Value {
    let email = invitation_email_from_context(row.context.as_ref()).unwrap_or_default();

    json!({
        "id": row.id,
        "email": email,
        "roleId": invitation_role_id_from_context(row.context.as_ref()),
        "expiresAt": row.expires_at.to_rfc3339(),
    })
}

async fn fetch_email_verification_token(
    state: &AppState,
    plain_token: &str,
) -> AuthResult<Option<EmailVerificationTokenRow>> {
    let row: Option<EmailVerificationTokenRow> = sqlx::query_as(
        "SELECT id, \"userId\", \"expiresAt\", context FROM core.\"appToken\" \
         WHERE value = $1 AND type = $2",
    )
    .bind(hash_token(plain_token))
    .bind(APP_TOKEN_TYPE_EMAIL_VERIFICATION)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

async fn validate_email_verification_token(
    state: &AppState,
    email_verification_token: &str,
    email: &str,
) -> AuthResult<EmailVerificationTokenRow> {
    let already_verified: Option<(bool,)> = sqlx::query_as(
        "SELECT \"isEmailVerified\" FROM core.\"user\" \
         WHERE email = $1 AND \"isEmailVerified\" = true",
    )
    .bind(email)
    .fetch_optional(&state.core_pool)
    .await?;

    if already_verified.is_some() {
        return Err(AuthError::new(
            400,
            "EMAIL_ALREADY_VERIFIED",
            "Email already verified",
        ));
    }

    let row = fetch_email_verification_token(state, email_verification_token)
        .await?
        .ok_or_else(|| AuthError::new(403, "INVALID_TOKEN", "Invalid email verification token"))?;

    if token_expired(row.expires_at, Utc::now()) {
        return Err(AuthError::new(
            403,
            "TOKEN_EXPIRED",
            "Email verification token expired",
        ));
    }

    let context_email = invitation_email_from_context(row.context.as_ref()).ok_or_else(|| {
        AuthError::new(
            400,
            "EMAIL_MISSING",
            "Email missing in email verification token context",
        )
    })?;

    if context_email != email {
        return Err(AuthError::new(
            400,
            "INVALID_EMAIL",
            "Email does not match token",
        ));
    }

    Ok(row)
}

async fn update_email_from_verification_token(
    state: &AppState,
    user_id: Uuid,
    email: &str,
) -> AuthResult<()> {
    sqlx::query("UPDATE core.\"user\" SET email = $2, \"updatedAt\" = now() WHERE id = $1")
        .bind(user_id)
        .bind(email)
        .execute(&state.core_pool)
        .await?;

    Ok(())
}

async fn mark_email_as_verified(state: &AppState, user_id: Uuid) -> AuthResult<()> {
    sqlx::query(
        "UPDATE core.\"user\" SET \"isEmailVerified\" = true, \"updatedAt\" = now() WHERE id = $1",
    )
    .bind(user_id)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

async fn consume_email_verification_token(
    state: &AppState,
    token_row: &EmailVerificationTokenRow,
    user_email: &str,
) -> AuthResult<String> {
    let context_email = invitation_email_from_context(token_row.context.as_ref());

    let user_email = match context_email {
        Some(context_email) if context_email != user_email => {
            update_email_from_verification_token(state, token_row.user_id, &context_email).await?;

            context_email
        }
        _ => user_email.to_string(),
    };

    mark_email_as_verified(state, token_row.user_id).await?;

    sqlx::query("DELETE FROM core.\"appToken\" WHERE id = $1")
        .bind(token_row.id)
        .execute(&state.core_pool)
        .await?;

    Ok(user_email)
}

async fn first_workspace_for_user(
    state: &AppState,
    user_id: Uuid,
) -> AuthResult<Option<(Uuid, Option<String>, Option<String>)>> {
    let row: Option<(Uuid, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT w.id, w.\"displayName\", w.subdomain \
         FROM core.\"workspace\" w \
         JOIN core.\"userWorkspace\" uw ON uw.\"workspaceId\" = w.id \
         WHERE uw.\"userId\" = $1 AND uw.\"deletedAt\" IS NULL AND w.\"deletedAt\" IS NULL \
         ORDER BY w.\"createdAt\" ASC, w.id ASC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

async fn get_one_workspace_invitation(
    state: &AppState,
    workspace_id: Uuid,
    email: &str,
) -> AuthResult<Option<AppTokenRow>> {
    let row: Option<AppTokenRow> = sqlx::query_as(
        "SELECT id, \"workspaceId\", \"expiresAt\", value, type, context \
         FROM core.\"appToken\" \
         WHERE \"workspaceId\" = $1 AND type = ANY($2) AND \"deletedAt\" IS NULL \
         AND \"expiresAt\" > now() AND lower(context->>'email') = lower($3)",
    )
    .bind(workspace_id)
    .bind(invitation_token_types())
    .bind(email)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

async fn fetch_workspace_invitation_row(
    state: &AppState,
    invitation_token: &str,
) -> AuthResult<Option<AppTokenRow>> {
    let row: Option<AppTokenRow> = sqlx::query_as(
        "SELECT id, \"workspaceId\", \"expiresAt\", value, type, context \
         FROM core.\"appToken\" WHERE value = $1 AND type = ANY($2)",
    )
    .bind(invitation_token)
    .bind(invitation_token_types())
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

async fn invalidate_workspace_invitation(
    state: &AppState,
    workspace_id: Uuid,
    email: &str,
) -> AuthResult<()> {
    let invitation = get_one_workspace_invitation(state, workspace_id, email).await?;

    if let Some(invitation) = invitation {
        sqlx::query("DELETE FROM core.\"appToken\" WHERE id = $1")
            .bind(invitation.id)
            .execute(&state.core_pool)
            .await?;
    }

    Ok(())
}

async fn add_user_to_workspace(
    state: &AppState,
    user_id: Uuid,
    user_email: &str,
    workspace_id: Uuid,
    invitation_role_id: Option<Uuid>,
) -> AuthResult<Uuid> {
    let membership: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    if let Some((user_workspace_id,)) = membership {
        return Ok(user_workspace_id);
    }

    let resolved_role_id = match invitation_role_id {
        Some(role_id) => role_id,
        None => {
            let row: Option<(Option<Uuid>,)> =
                sqlx::query_as("SELECT \"defaultRoleId\" FROM core.\"workspace\" WHERE id = $1")
                    .bind(workspace_id)
                    .fetch_optional(&state.core_pool)
                    .await?;

            row.and_then(|(role_id,)| role_id)
                .ok_or_else(|| AuthError::forbidden("Default role not found"))?
        }
    };

    let assignable: Option<(bool,)> =
        sqlx::query_as("SELECT \"canBeAssignedToUsers\" FROM core.role WHERE id = $1")
            .bind(resolved_role_id)
            .fetch_optional(&state.core_pool)
            .await?;

    let assignable = assignable.map(|(assignable,)| assignable).unwrap_or(false);

    if !assignable {
        return Err(AuthError::forbidden("Role cannot be assigned to users"));
    }

    let locale: Option<(String,)> =
        sqlx::query_as("SELECT locale FROM core.\"user\" WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.core_pool)
            .await?;

    let user_workspace_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO core.\"userWorkspace\" (\"userId\", \"workspaceId\", locale) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(user_id)
    .bind(workspace_id)
    .bind(
        locale
            .map(|(locale,)| locale)
            .unwrap_or_else(|| "en".to_string()),
    )
    .fetch_one(&state.core_pool)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"roleTarget\" \
         (\"workspaceId\", \"roleId\", \"userWorkspaceId\", \"universalIdentifier\", \"applicationId\") \
         SELECT $1, $2, $3, $4, w.\"workspaceCustomApplicationId\" \
         FROM core.\"workspace\" w WHERE w.id = $1",
    )
    .bind(workspace_id)
    .bind(resolved_role_id)
    .bind(user_workspace_id.0)
    .bind(Uuid::new_v4())
    .execute(&state.core_pool)
    .await?;

    invalidate_workspace_invitation(state, workspace_id, user_email).await?;

    Ok(user_workspace_id.0)
}

async fn join_workspace_if_invited(
    state: &AppState,
    user_id: Uuid,
    user_email: &str,
    workspace_id: Uuid,
) -> AuthResult<()> {
    let invitation = get_one_workspace_invitation(state, workspace_id, user_email).await?;

    let Some(invitation) = invitation else {
        return Ok(());
    };

    add_user_to_workspace(
        state,
        user_id,
        user_email,
        workspace_id,
        invitation_role_id_from_context(invitation.context.as_ref()),
    )
    .await?;

    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyEmailInput {
    email_verification_token: String,
    email: String,
    #[serde(default)]
    workspace_id: Option<Uuid>,
}

async fn verify_email_login_token(
    State(state): State<AppState>,
    Json(input): Json<VerifyEmailInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let token_row =
        validate_email_verification_token(&state, &input.email_verification_token, &input.email)
            .await?;

    let user_email = consume_email_verification_token(&state, &token_row, &input.email).await?;
    let user_id = token_row.user_id;

    let (workspace_id, display_name, subdomain) = match input.workspace_id {
        Some(workspace_id) => {
            let row: Option<(Uuid, Option<String>, Option<String>)> = sqlx::query_as(
                "SELECT id, \"displayName\", subdomain FROM core.\"workspace\" \
                 WHERE id = $1 AND \"deletedAt\" IS NULL",
            )
            .bind(workspace_id)
            .fetch_optional(&state.core_pool)
            .await?;

            row.ok_or_else(|| AuthError::workspace_not_found("Workspace not found"))?
        }
        None => first_workspace_for_user(&state, user_id)
            .await?
            .ok_or_else(|| AuthError::workspace_not_found("Workspace not found"))?,
    };

    join_workspace_if_invited(&state, user_id, &user_email, workspace_id).await?;

    let login_token =
        tokens::issue_login_token(&state, &user_email, workspace_id, AuthProvider::Password)
            .await?;

    let expires_at =
        Utc::now() + Duration::seconds(state.config.login_token_expires_in_secs as i64);

    Ok(Json(json!({
        "loginToken": {
            "token": login_token,
            "expiresAt": expires_at.to_rfc3339(),
        },
        "workspaceUrls": {
            "subdomainUrl": subdomain_url(&state.config, subdomain.as_deref()),
        },
        "workspace": {
            "id": workspace_id,
            "displayName": display_name,
        },
        "user": {
            "id": user_id,
            "email": user_email,
        },
    })))
}

async fn verify_email_workspace_agnostic_token(
    State(state): State<AppState>,
    Json(input): Json<VerifyEmailInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let token_row =
        validate_email_verification_token(&state, &input.email_verification_token, &input.email)
            .await?;

    let user_email = consume_email_verification_token(&state, &token_row, &input.email).await?;
    let user_id = token_row.user_id;

    let memberships: Vec<(Uuid, Option<String>, Option<String>, bool)> = sqlx::query_as(
        "SELECT w.id, w.\"displayName\", w.subdomain, w.\"isPasswordAuthEnabled\" \
         FROM core.\"workspace\" w \
         JOIN core.\"userWorkspace\" uw ON uw.\"workspaceId\" = w.id \
         WHERE uw.\"userId\" = $1 AND uw.\"deletedAt\" IS NULL \
         AND w.\"deletedAt\" IS NULL AND w.\"workspaceDiscoverability\" != 'HIDDEN' \
         ORDER BY w.\"createdAt\" ASC, w.id ASC",
    )
    .bind(user_id)
    .fetch_all(&state.core_pool)
    .await?;

    let mut available_workspaces_for_sign_in = Vec::with_capacity(memberships.len());

    for (workspace_id, display_name, subdomain, password_auth_enabled) in memberships {
        let mut entry = json!({
            "id": workspace_id,
            "displayName": display_name,
            "workspaceUrls": {
                "subdomainUrl": subdomain_url(&state.config, subdomain.as_deref()),
            },
            "sso": [],
        });

        if password_auth_enabled {
            let login_token = tokens::issue_login_token(
                &state,
                &user_email,
                workspace_id,
                AuthProvider::Password,
            )
            .await?;

            entry["loginToken"] = Value::String(login_token);
        }

        available_workspaces_for_sign_in.push(entry);
    }

    let invitations: Vec<AppTokenRow> = sqlx::query_as(
        "SELECT id, \"workspaceId\", \"expiresAt\", value, type, context \
         FROM core.\"appToken\" \
         WHERE type = ANY($1) AND \"deletedAt\" IS NULL AND \"expiresAt\" > now() \
         AND lower(context->>'email') = lower($2)",
    )
    .bind(invitation_token_types())
    .bind(&user_email)
    .fetch_all(&state.core_pool)
    .await?;

    let mut available_workspaces_for_sign_up = Vec::with_capacity(invitations.len());

    for invitation in invitations {
        let workspace: Option<(Uuid, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT id, \"displayName\", subdomain FROM core.\"workspace\" \
             WHERE id = $1 AND \"deletedAt\" IS NULL",
        )
        .bind(invitation.workspace_id)
        .fetch_optional(&state.core_pool)
        .await?;

        if let Some((workspace_id, display_name, subdomain)) = workspace {
            available_workspaces_for_sign_up.push(json!({
                "id": workspace_id,
                "displayName": display_name,
                "personalInviteToken": invitation.value,
                "workspaceUrls": {
                    "subdomainUrl": subdomain_url(&state.config, subdomain.as_deref()),
                },
                "sso": [],
            }));
        }
    }

    let access_token =
        tokens::issue_workspace_agnostic_token(&state, user_id, AuthProvider::Password).await?;

    let refresh_token = tokens::issue_refresh_token(
        &state,
        user_id,
        None,
        AuthProvider::Password,
        TokenType::WorkspaceAgnostic,
    )
    .await?;

    let access_expires_at = Utc::now()
        + Duration::seconds(state.config.workspace_agnostic_token_expires_in_secs as i64);
    let refresh_expires_at =
        Utc::now() + Duration::seconds(state.config.refresh_token_expires_in_secs as i64);

    Ok(Json(json!({
        "availableWorkspaces": {
            "availableWorkspacesForSignIn": available_workspaces_for_sign_in,
            "availableWorkspacesForSignUp": available_workspaces_for_sign_up,
        },
        "tokens": {
            "accessOrWorkspaceAgnosticToken": {
                "token": access_token,
                "expiresAt": access_expires_at.to_rfc3339(),
            },
            "refreshToken": {
                "token": refresh_token,
                "expiresAt": refresh_expires_at.to_rfc3339(),
            },
        },
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResendEmailVerificationTokenInput {
    email: String,
}

async fn resend_email_verification_token(
    State(state): State<AppState>,
    Json(input): Json<ResendEmailVerificationTokenInput>,
) -> AuthResult<Json<serde_json::Value>> {
    if !is_email_verification_required() {
        return Err(AuthError::new(
            400,
            "EMAIL_VERIFICATION_NOT_REQUIRED",
            "Email verification token cannot be sent because email verification is not required",
        ));
    }

    let email = super::service::normalize_email(&input.email)?;

    let user: Option<(Uuid, bool)> =
        sqlx::query_as("SELECT id, \"isEmailVerified\" FROM core.\"user\" WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.core_pool)
            .await?;

    let (user_id, is_verified) = user.ok_or_else(|| AuthError::internal("Value not defined"))?;

    if is_verified {
        return Err(AuthError::new(
            400,
            "EMAIL_ALREADY_VERIFIED",
            "Email already verified",
        ));
    }

    let latest: Option<(DateTime<Utc>,)> = sqlx::query_as(
        "SELECT \"createdAt\" FROM core.\"appToken\" \
         WHERE \"userId\" = $1 AND type = $2 \
         ORDER BY \"createdAt\" DESC LIMIT 1",
    )
    .bind(user_id)
    .bind(APP_TOKEN_TYPE_EMAIL_VERIFICATION)
    .fetch_optional(&state.core_pool)
    .await?;

    if let Some((created_at,)) = latest {
        let elapsed = (Utc::now() - created_at).num_seconds();

        if elapsed < EMAIL_VERIFICATION_RESEND_COOLDOWN_SECS {
            return Err(AuthError::new(
                403,
                "RATE_LIMIT_EXCEEDED",
                cooldown_wait_message(EMAIL_VERIFICATION_RESEND_COOLDOWN_SECS - elapsed),
            ));
        }
    }

    let count = consume_bucket(
        &state,
        &format!("{VERIFICATION_EMAIL_THROTTLE_PREFIX}{email}"),
        1,
        VERIFICATION_EMAIL_SENDING_WINDOW_SECS,
    )
    .await?;

    if !bucket_admits(count, VERIFICATION_EMAIL_SENDING_LIMIT) {
        return Err(AuthError::new(
            403,
            "RATE_LIMIT_EXCEEDED",
            "Verification email sending rate limit exceeded",
        ));
    }

    sqlx::query("DELETE FROM core.\"appToken\" WHERE \"userId\" = $1 AND type = $2")
        .bind(user_id)
        .bind(APP_TOKEN_TYPE_EMAIL_VERIFICATION)
        .execute(&state.core_pool)
        .await?;

    let plain_token = generate_plain_token();
    let expires_at = Utc::now() + Duration::seconds(EMAIL_VERIFICATION_TOKEN_EXPIRES_IN_SECS);

    sqlx::query(
        "INSERT INTO core.\"appToken\" (\"userId\", type, value, \"expiresAt\", context) \
         VALUES ($1, $2, $3, $4, $5::jsonb)",
    )
    .bind(user_id)
    .bind(APP_TOKEN_TYPE_EMAIL_VERIFICATION)
    .bind(hash_token(&plain_token))
    .bind(expires_at)
    .bind(json!({ "email": email }).to_string())
    .execute(&state.core_pool)
    .await?;

    let link = verification_link(&state.config.frontend_url, &plain_token, &email);

    tracing::info!(
        to = %email,
        subject = "Welcome to Nudge: Please Confirm Your Email",
        link = %link,
        "email verification link (EMAIL_DRIVER=LOGGER)"
    );

    Ok(Json(json!({ "success": true })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmailPasswordResetLinkInput {
    email: String,
    #[serde(default)]
    workspace_id: Option<Uuid>,
}

struct PasswordResetTarget {
    user_id: Uuid,
    email: String,
    has_password: bool,
    workspace_id: Uuid,
    locale: Option<String>,
}

async fn find_first_password_auth_workspace(
    state: &AppState,
    user_id: Uuid,
) -> AuthResult<Option<(Uuid, Option<String>)>> {
    let row: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT w.id, uw.locale FROM core.\"workspace\" w \
         JOIN core.\"userWorkspace\" uw ON uw.\"workspaceId\" = w.id \
         WHERE uw.\"userId\" = $1 AND w.\"isPasswordAuthEnabled\" = true \
         AND w.\"deletedAt\" IS NULL AND uw.\"deletedAt\" IS NULL \
         ORDER BY w.\"createdAt\" ASC, w.id ASC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

async fn resolve_password_reset_target(
    state: &AppState,
    email: &str,
    requested_workspace_id: Option<Uuid>,
) -> AuthResult<Option<PasswordResetTarget>> {
    let user: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, \"passwordHash\" FROM core.\"user\" \
         WHERE email = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(email)
    .fetch_optional(&state.core_pool)
    .await?;

    let Some((user_id, password_hash)) = user else {
        return Ok(None);
    };

    let target = match requested_workspace_id {
        Some(workspace_id) => {
            let row: Option<(Uuid, Option<String>)> = sqlx::query_as(
                "SELECT w.id, uw.locale FROM core.\"workspace\" w \
                 JOIN core.\"userWorkspace\" uw ON uw.\"workspaceId\" = w.id \
                 WHERE w.id = $1 AND uw.\"userId\" = $2 \
                 AND w.\"isPasswordAuthEnabled\" = true \
                 AND w.\"deletedAt\" IS NULL AND uw.\"deletedAt\" IS NULL",
            )
            .bind(workspace_id)
            .bind(user_id)
            .fetch_optional(&state.core_pool)
            .await?;

            match row {
                Some(target) => Some(target),
                None => find_first_password_auth_workspace(state, user_id).await?,
            }
        }
        None => find_first_password_auth_workspace(state, user_id).await?,
    };

    let Some((workspace_id, locale)) = target else {
        return Ok(None);
    };

    Ok(Some(PasswordResetTarget {
        user_id,
        email: email.to_string(),
        has_password: password_hash.is_some(),
        workspace_id,
        locale,
    }))
}

async fn rotate_password_reset_token(
    state: &AppState,
    target: &PasswordResetTarget,
    plain_token: &str,
    expires_at: DateTime<Utc>,
) -> AuthResult<()> {
    let mut transaction = state.core_pool.begin().await?;

    sqlx::query(
        "UPDATE core.\"appToken\" SET \"revokedAt\" = now(), \"updatedAt\" = now() \
         WHERE \"userId\" = $1 AND type = $2",
    )
    .bind(target.user_id)
    .bind(APP_TOKEN_TYPE_PASSWORD_RESET)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"appToken\" \
         (\"userId\", \"workspaceId\", type, value, \"expiresAt\") \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(target.user_id)
    .bind(target.workspace_id)
    .bind(APP_TOKEN_TYPE_PASSWORD_RESET)
    .bind(hash_token(plain_token))
    .bind(expires_at)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(())
}

async fn email_password_reset_link(
    State(state): State<AppState>,
    Json(input): Json<EmailPasswordResetLinkInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let email = super::service::normalize_email(&input.email)?;

    let count = consume_bucket(
        &state,
        &format!("{PASSWORD_RESET_EMAIL_THROTTLE_PREFIX}{email}"),
        1,
        PASSWORD_RESET_EMAIL_RATE_LIMIT_WINDOW_SECS,
    )
    .await?;

    if !bucket_admits(count, PASSWORD_RESET_EMAIL_RATE_LIMIT_MAX) {
        return Err(limit_reached_error());
    }

    match resolve_password_reset_target(&state, &email, input.workspace_id).await? {
        Some(target) => {
            let plain_token = generate_plain_token();
            let expires_at = Utc::now() + Duration::seconds(PASSWORD_RESET_TOKEN_EXPIRES_IN_SECS);

            rotate_password_reset_token(&state, &target, &plain_token, expires_at).await?;

            let link = reset_password_link(&state.config.frontend_url, &plain_token);
            let subject = password_reset_subject(target.has_password);

            tracing::info!(
                to = %target.email,
                subject = subject,
                link = %link,
                expiresAt = %expires_at.to_rfc3339(),
                locale = target.locale.as_deref().unwrap_or("en"),
                "password reset link email (EMAIL_DRIVER=LOGGER)"
            );
        }
        None => {
            tracing::warn!(
                %email,
                "password reset request silently ignored: no user or no password auth workspace"
            );
        }
    }

    Ok(Json(json!({ "success": true })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidatePasswordResetTokenInput {
    password_reset_token: String,
}

async fn find_live_password_reset_token(
    state: &AppState,
    plain_token: &str,
) -> AuthResult<Option<Uuid>> {
    let row: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT \"userId\" FROM core.\"appToken\" \
         WHERE value = $1 AND type = $2 AND \"expiresAt\" > now() AND \"revokedAt\" IS NULL",
    )
    .bind(hash_token(plain_token))
    .bind(APP_TOKEN_TYPE_PASSWORD_RESET)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row.and_then(|(user_id,)| user_id))
}

fn invalid_password_reset_token_error() -> AuthError {
    AuthError::forbidden("Token is invalid")
}

async fn validate_password_reset_token(
    State(state): State<AppState>,
    Json(input): Json<ValidatePasswordResetTokenInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let user_id = find_live_password_reset_token(&state, &input.password_reset_token)
        .await?
        .ok_or_else(invalid_password_reset_token_error)?;

    let user: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
        "SELECT id, email::text, \"passwordHash\" FROM core.\"user\" \
         WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (id, email, password_hash) =
        user.ok_or_else(|| AuthError::invalid_input("User not found"))?;

    Ok(Json(json!({
        "id": id,
        "email": email,
        "hasPassword": password_hash.is_some(),
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePasswordViaResetTokenInput {
    password_reset_token: String,
    new_password: String,
}

async fn update_password_via_reset_token(
    State(state): State<AppState>,
    Json(input): Json<UpdatePasswordViaResetTokenInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let user_id = find_live_password_reset_token(&state, &input.password_reset_token)
        .await?
        .ok_or_else(invalid_password_reset_token_error)?;

    update_password(&state, user_id, &input.new_password).await?;

    sqlx::query(
        "UPDATE core.\"appToken\" SET \"revokedAt\" = now(), \"updatedAt\" = now() \
         WHERE \"userId\" = $1 AND type = $2",
    )
    .bind(user_id)
    .bind(APP_TOKEN_TYPE_PASSWORD_RESET)
    .execute(&state.core_pool)
    .await?;

    Ok(Json(json!({ "success": true })))
}

async fn update_password(state: &AppState, user_id: Uuid, new_password: &str) -> AuthResult<()> {
    let user: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT email::text, \"firstName\", \"lastName\" FROM core.\"user\" \
         WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (email, first_name, last_name) =
        user.ok_or_else(|| AuthError::user_not_found("User not found"))?;

    let locale: Option<(String,)> = sqlx::query_as(
        "SELECT locale FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"deletedAt\" IS NULL \
         ORDER BY \"createdAt\" ASC, id ASC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?;

    if locale.is_none() {
        return Err(AuthError::new(
            500,
            "USER_WORKSPACE_NOT_FOUND",
            "User does not have a workspace",
        ));
    }

    if !super::crypto::is_valid_password(new_password) {
        return Err(AuthError::invalid_input("Password is too weak"));
    }

    let password_hash = super::crypto::hash_password(new_password)
        .map_err(|error| AuthError::internal(error.to_string()))?;

    sqlx::query(
        "UPDATE core.\"user\" SET \"passwordHash\" = $2, \"updatedAt\" = now() WHERE id = $1",
    )
    .bind(user_id)
    .bind(&password_hash)
    .execute(&state.core_pool)
    .await?;

    sqlx::query(
        "UPDATE core.\"appToken\" SET \"revokedAt\" = now(), \"updatedAt\" = now() \
         WHERE \"userId\" = $1 AND type = 'REFRESH_TOKEN' AND \"revokedAt\" IS NULL",
    )
    .bind(user_id)
    .execute(&state.core_pool)
    .await?;

    session::revoke_all_sessions_for_user(
        state,
        user_id,
        None,
        session::RevocationReason::PasswordChanged,
        None,
    )
    .await?;

    let display_name = format!(
        "{} {}",
        first_name.unwrap_or_default(),
        last_name.unwrap_or_default()
    )
    .trim()
    .to_string();

    tracing::info!(
        to = %email,
        subject = "Your Password Has Been Successfully Changed",
        user = %display_name,
        "password changed notification email (EMAIL_DRIVER=LOGGER)"
    );

    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendInvitationsInput {
    emails: Vec<String>,
    #[serde(default)]
    role_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppTokenIdInput {
    app_token_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcceptInvitationInput {
    invitation_token: String,
    email: String,
}

async fn assert_role_assignable_to_users(
    state: &AppState,
    workspace_id: Uuid,
    role_id: Uuid,
) -> AuthResult<()> {
    let role: Option<(bool,)> = sqlx::query_as(
        "SELECT \"canBeAssignedToUsers\" FROM core.role \
         WHERE id = $1 AND \"workspaceId\" = $2",
    )
    .bind(role_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let assignable = role
        .map(|(assignable,)| assignable)
        .ok_or_else(|| AuthError::invalid_input("Role not found in this workspace"))?;

    if !assignable {
        return Err(AuthError::forbidden("Role cannot be assigned to users"));
    }

    Ok(())
}

fn invitation_rate_limit_error() -> AuthError {
    AuthError::new(
        403,
        "INVALID_INVITATION",
        "Workspace invitation sending rate limit exceeded.",
    )
}

fn is_same_work_domain(sender_email: &str, invitee_email: &str) -> bool {
    match (
        sender_email.rsplit_once('@'),
        invitee_email.rsplit_once('@'),
    ) {
        (Some((_, sender_domain)), Some((_, invitee_domain))) => {
            sender_domain.eq_ignore_ascii_case(invitee_domain)
        }
        _ => false,
    }
}

async fn throttle_invitation_sending(
    state: &AppState,
    workspace_id: Uuid,
    sender_user_id: Uuid,
    sender_email: Option<&str>,
    emails: &[String],
) -> AuthResult<()> {
    for email in emails {
        let count = consume_bucket(
            state,
            &format!("{INVITATION_SENDING_THROTTLE_PREFIX}{email}"),
            1,
            INVITATION_SENDING_BY_EMAIL_WINDOW_SECS,
        )
        .await?;

        if !bucket_admits(count, INVITATION_SENDING_BY_EMAIL_LIMIT) {
            return Err(invitation_rate_limit_error());
        }
    }

    let same_work_domain_count = match sender_email {
        Some(sender_email) => emails
            .iter()
            .filter(|email| is_same_work_domain(sender_email, email))
            .count() as i64,
        None => 0,
    };

    let count = consume_bucket(
        state,
        &format!("{INVITATION_SENDING_THROTTLE_PREFIX}{sender_user_id}:same-work-domain"),
        same_work_domain_count,
        INVITATION_SENDING_BY_SENDER_WINDOW_SECS,
    )
    .await?;

    if !bucket_admits(count, INVITATION_SENDING_BY_SENDER_SAME_WORK_DOMAIN_LIMIT) {
        return Err(invitation_rate_limit_error());
    }

    let count = consume_bucket(
        state,
        &format!("{INVITATION_SENDING_THROTTLE_PREFIX}{sender_user_id}"),
        emails.len() as i64 - same_work_domain_count,
        INVITATION_SENDING_BY_SENDER_WINDOW_SECS,
    )
    .await?;

    if !bucket_admits(count, INVITATION_SENDING_BY_SENDER_LIMIT) {
        return Err(invitation_rate_limit_error());
    }

    let count = consume_bucket(
        state,
        &format!("{INVITATION_SENDING_THROTTLE_PREFIX}{workspace_id}"),
        emails.len() as i64,
        INVITATION_SENDING_BY_WORKSPACE_WINDOW_SECS,
    )
    .await?;

    if !bucket_admits(count, INVITATION_SENDING_BY_WORKSPACE_LIMIT) {
        return Err(invitation_rate_limit_error());
    }

    Ok(())
}

struct InvitationOutcome {
    row: AppTokenRow,
}

async fn create_workspace_invitation(
    state: &AppState,
    workspace_id: Uuid,
    workspace_display_name: Option<&str>,
    email: &str,
    role_id: Option<Uuid>,
    is_onboarding_invitation: bool,
) -> AuthResult<InvitationOutcome> {
    let existing = get_one_workspace_invitation(state, workspace_id, email).await?;

    if existing.is_some() {
        return Err(AuthError::new(
            400,
            "INVITATION_ALREADY_EXIST",
            format!("{email} already invited"),
        ));
    }

    let already_member: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM core.\"userWorkspace\" uw \
         JOIN core.\"user\" u ON u.id = uw.\"userId\" \
         WHERE uw.\"workspaceId\" = $1 AND u.email = $2 \
         AND uw.\"deletedAt\" IS NULL)",
    )
    .bind(workspace_id)
    .bind(email)
    .fetch_optional(&state.core_pool)
    .await?;

    if already_member.map(|(exists,)| exists).unwrap_or(false) {
        return Err(AuthError::new(
            400,
            "USER_ALREADY_EXIST",
            format!("{email} is already in the workspace"),
        ));
    }

    sqlx::query(
        "DELETE FROM core.\"appToken\" \
         WHERE \"workspaceId\" = $1 AND type = ANY($2) AND \"expiresAt\" <= now() \
         AND lower(context->>'email') = lower($3)",
    )
    .bind(workspace_id)
    .bind(invitation_token_types())
    .bind(email)
    .execute(&state.core_pool)
    .await?;

    let token_type = if is_onboarding_invitation {
        APP_TOKEN_TYPE_ONBOARDING_INVITATION
    } else {
        APP_TOKEN_TYPE_INVITATION
    };

    let plain_token = generate_plain_token();
    let expires_at = Utc::now() + Duration::seconds(INVITATION_TOKEN_EXPIRES_IN_SECS);

    let mut context = json!({ "email": email });

    if let Some(role_id) = role_id {
        context["roleId"] = Value::String(role_id.to_string());
    }

    let row: AppTokenRow = sqlx::query_as(
        "INSERT INTO core.\"appToken\" \
         (\"workspaceId\", type, value, \"expiresAt\", context) \
         VALUES ($1, $2, $3, $4, $5::jsonb) \
         RETURNING id, \"workspaceId\", \"expiresAt\", value, type, context",
    )
    .bind(workspace_id)
    .bind(token_type)
    .bind(&plain_token)
    .bind(expires_at)
    .bind(context.to_string())
    .fetch_one(&state.core_pool)
    .await?;

    let link = invitation_link(&state.config.frontend_url, &plain_token, email);

    tracing::info!(
        to = %email,
        subject = "Join your team on Nudge",
        workspace = workspace_display_name.unwrap_or("Workspace"),
        link = %link,
        "workspace invitation email (EMAIL_DRIVER=LOGGER)"
    );

    Ok(InvitationOutcome { row })
}

struct SendInvitationsCommand {
    workspace_id: Uuid,
    sender_user_id: Uuid,
    sender_email: Option<String>,
    emails: Vec<String>,
    role_id: Option<Uuid>,
    app_token_id_to_invalidate: Option<Uuid>,
    is_onboarding_invite_reward_override: bool,
}

async fn send_invitations_inner(
    state: &AppState,
    command: SendInvitationsCommand,
) -> AuthResult<serde_json::Value> {
    if let Some(role_id) = command.role_id {
        assert_role_assignable_to_users(state, command.workspace_id, role_id).await?;
    }

    let workspace: Option<(Option<String>,)> =
        sqlx::query_as("SELECT \"displayName\" FROM core.\"workspace\" WHERE id = $1")
            .bind(command.workspace_id)
            .fetch_optional(&state.core_pool)
            .await?;

    let workspace_display_name = workspace.and_then(|(name,)| name);

    throttle_invitation_sending(
        state,
        command.workspace_id,
        command.sender_user_id,
        command.sender_email.as_deref(),
        &command.emails,
    )
    .await?;

    if let Some(app_token_id) = command.app_token_id_to_invalidate {
        sqlx::query("DELETE FROM core.\"appToken\" WHERE id = $1 AND type = ANY($2)")
            .bind(app_token_id)
            .bind(invitation_token_types())
            .execute(&state.core_pool)
            .await?;
    }

    let mut errors: Vec<String> = Vec::new();
    let mut result: Vec<Value> = Vec::new();

    for email in command.emails {
        match create_workspace_invitation(
            state,
            command.workspace_id,
            workspace_display_name.as_deref(),
            &email,
            command.role_id,
            command.is_onboarding_invite_reward_override,
        )
        .await
        {
            Ok(outcome) => result.push(invitation_json(&outcome.row)),
            Err(error) => errors.push(error.to_string()),
        }
    }

    Ok(json!({
        "success": errors.is_empty(),
        "errors": errors,
        "result": result,
    }))
}

async fn require_workspace_membership(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<()> {
    let member: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    member
        .map(|_| ())
        .ok_or_else(|| AuthError::forbidden("User is not a member of the workspace."))
}

async fn sender_email_for_user(state: &AppState, user_id: Uuid) -> AuthResult<String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT email::text FROM core.\"user\" WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.core_pool)
            .await?;

    row.map(|(email,)| email)
        .ok_or_else(|| AuthError::user_not_found("User not found"))
}

async fn send_invitations(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<SendInvitationsInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let workspace_id = super::api_keys::workspace_id_from(&context)?;
    let user_id = context
        .user_id
        .ok_or_else(|| AuthError::forbidden("This endpoint requires a user session"))?;

    require_workspace_membership(&state, user_id, workspace_id).await?;

    let emails = validate_invitation_emails(&input.emails)?;
    let sender_email = sender_email_for_user(&state, user_id).await?;

    send_invitations_inner(
        &state,
        SendInvitationsCommand {
            workspace_id,
            sender_user_id: user_id,
            sender_email: Some(sender_email),
            emails,
            role_id: input.role_id,
            app_token_id_to_invalidate: None,
            is_onboarding_invite_reward_override: false,
        },
    )
    .await
    .map(Json)
}

async fn find_workspace_invitations(
    State(state): State<AppState>,
    context: AuthContext,
) -> AuthResult<Json<serde_json::Value>> {
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let rows: Vec<AppTokenRow> = sqlx::query_as(
        "SELECT id, \"workspaceId\", \"expiresAt\", value, type, context \
         FROM core.\"appToken\" \
         WHERE \"workspaceId\" = $1 AND type = ANY($2) AND \"deletedAt\" IS NULL \
         ORDER BY \"createdAt\" ASC, id ASC",
    )
    .bind(workspace_id)
    .bind(invitation_token_types())
    .fetch_all(&state.core_pool)
    .await?;

    let invitations: Vec<Value> = rows.iter().map(invitation_json).collect();

    Ok(Json(json!({ "invitations": invitations })))
}

async fn resend_workspace_invitation(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<AppTokenIdInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let workspace_id = super::api_keys::workspace_id_from(&context)?;
    let user_id = context
        .user_id
        .ok_or_else(|| AuthError::forbidden("This endpoint requires a user session"))?;

    require_workspace_membership(&state, user_id, workspace_id).await?;

    let row: Option<AppTokenRow> = sqlx::query_as(
        "SELECT id, \"workspaceId\", \"expiresAt\", value, type, context \
         FROM core.\"appToken\" \
         WHERE id = $1 AND \"workspaceId\" = $2 AND type = ANY($3)",
    )
    .bind(input.app_token_id)
    .bind(workspace_id)
    .bind(invitation_token_types())
    .fetch_optional(&state.core_pool)
    .await?;

    let invitation =
        row.ok_or_else(|| AuthError::new(403, "INVALID_INVITATION", "Invalid appToken"))?;

    let email = invitation_email_from_context(invitation.context.as_ref())
        .ok_or_else(|| AuthError::new(403, "INVALID_INVITATION", "Invalid appToken"))?;

    let sender_email = sender_email_for_user(&state, user_id).await?;

    send_invitations_inner(
        &state,
        SendInvitationsCommand {
            workspace_id,
            sender_user_id: user_id,
            sender_email: Some(sender_email),
            emails: vec![email],
            role_id: invitation_role_id_from_context(invitation.context.as_ref()),
            app_token_id_to_invalidate: Some(invitation.id),
            is_onboarding_invite_reward_override: invitation.r#type
                == APP_TOKEN_TYPE_ONBOARDING_INVITATION,
        },
    )
    .await
    .map(Json)
}

async fn delete_workspace_invitation(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<AppTokenIdInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let deleted = sqlx::query(
        "DELETE FROM core.\"appToken\" \
         WHERE id = $1 AND \"workspaceId\" = $2 AND type = ANY($3)",
    )
    .bind(input.app_token_id)
    .bind(workspace_id)
    .bind(invitation_token_types())
    .execute(&state.core_pool)
    .await?
    .rows_affected();

    if deleted == 0 {
        return Ok(Json(json!("error")));
    }

    Ok(Json(json!("success")))
}

async fn accept_workspace_invitation(
    State(state): State<AppState>,
    Json(input): Json<AcceptInvitationInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let invitation = fetch_workspace_invitation_row(&state, &input.invitation_token)
        .await?
        .ok_or_else(|| AuthError::forbidden("Invalid invitation token"))?;

    let context_email = invitation_email_from_context(invitation.context.as_ref())
        .filter(|email| !email.is_empty())
        .ok_or_else(|| AuthError::forbidden("Email does not match the invitation"))?;

    if !invitation_emails_match(&context_email, &input.email) {
        return Err(AuthError::forbidden("Email does not match the invitation"));
    }

    if token_expired(invitation.expires_at, Utc::now()) {
        return Err(AuthError::forbidden("Invitation expired"));
    }

    let workspace_id = invitation
        .workspace_id
        .ok_or_else(|| AuthError::forbidden("Invalid invitation token"))?;

    let normalized_email = super::service::normalize_email(&input.email)?;

    let user: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.\"user\" WHERE email = $1 AND \"deletedAt\" IS NULL")
            .bind(&normalized_email)
            .fetch_optional(&state.core_pool)
            .await?;

    let (user_id,) = user.ok_or_else(|| AuthError::user_not_found("User not found"))?;

    let role_id = invitation_role_id_from_context(invitation.context.as_ref());

    let user_workspace_id =
        add_user_to_workspace(&state, user_id, &normalized_email, workspace_id, role_id).await?;

    let workspace: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT \"displayName\", subdomain FROM core.\"workspace\" WHERE id = $1")
            .bind(workspace_id)
            .fetch_optional(&state.core_pool)
            .await?;

    let (display_name, subdomain) = workspace.unwrap_or((None, None));

    Ok(Json(json!({
        "workspace": {
            "id": workspace_id,
            "displayName": display_name,
            "subdomain": subdomain,
        },
        "userWorkspaceId": user_workspace_id,
        "roleId": role_id,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InviteHashInput {
    invite_hash: String,
}

async fn workspace_invite_hash_valid(
    State(state): State<AppState>,
    Json(input): Json<InviteHashInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.\"workspace\" WHERE \"inviteHash\" = $1")
            .bind(&input.invite_hash)
            .fetch_optional(&state.core_pool)
            .await?;

    Ok(Json(json!({ "isValid": row.is_some() })))
}

async fn workspace_from_invite_hash(
    State(state): State<AppState>,
    Json(input): Json<InviteHashInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let row: Option<(Uuid, Option<String>, bool)> = sqlx::query_as(
        "SELECT id, \"displayName\", \"allowImpersonation\" FROM core.\"workspace\" \
         WHERE \"inviteHash\" = $1",
    )
    .bind(&input.invite_hash)
    .fetch_optional(&state.core_pool)
    .await?;

    let (id, display_name, allow_impersonation) =
        row.ok_or_else(|| AuthError::invalid_input("Workspace does not exist."))?;

    Ok(Json(json!({
        "id": id,
        "displayName": display_name,
        "logo": "",
        "allowImpersonation": allow_impersonation,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_fixture() -> nudge_core::Config {
        nudge_core::Config {
            node_env: nudge_core::NodeEnv::Development,
            node_port: 3000,
            server_url: "http://localhost:3000".to_string(),
            pg_database_url: "postgres://localhost/default".to_string(),
            pg_pool_max_connections: 10,
            redis_url: "redis://127.0.0.1:6379".to_string(),
            redis_cache_prefix: String::new(),
            app_secret: "secret".to_string(),
            encryption_key: None,
            fallback_encryption_key: None,
            access_token_expires_in_secs: 1800,
            refresh_token_expires_in_secs: 60 * 24 * 60 * 60,
            login_token_expires_in_secs: 900,
            workspace_agnostic_token_expires_in_secs: 1800,
            refresh_token_reuse_grace_period_secs: 60,
            frontend_url: "http://localhost:3001".to_string(),
            auth_cookie_same_site: "lax".to_string(),
            auth_cookie_allowed_origins: String::new(),
            session_absolute_lifetime_secs: 180 * 24 * 60 * 60,
            session_idle_timeout_secs: 30 * 24 * 60 * 60,
            is_cloudflare_geolocation_enabled: false,
            workspace_schema_ddl_locked: false,
            search_ilike_fallback_timeout_ms: 2000,
        }
    }

    #[test]
    fn token_hashes_match_the_node_sha256_hex_contract() {
        assert_eq!(
            hash_token(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash_token("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }

    #[test]
    fn generated_tokens_are_64_hex_characters() {
        let token = generate_plain_token();

        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(generate_plain_token(), generate_plain_token());
    }

    #[test]
    fn invitation_tokens_cover_both_upstream_app_token_types() {
        let types = invitation_token_types();

        assert_eq!(types.len(), 2);
        assert!(types.contains(&"INVITATION_TOKEN"));
        assert!(types.contains(&"ONBOARDING_INVITATION_TOKEN"));
    }

    #[test]
    fn invitation_emails_compare_case_insensitively() {
        assert!(invitation_emails_match(
            "User@Example.com",
            "user@example.com"
        ));
        assert!(invitation_emails_match(
            "user@example.com",
            "USER@EXAMPLE.COM"
        ));
        assert!(!invitation_emails_match(
            "user@example.com",
            "other@example.com"
        ));
        assert!(!invitation_emails_match(
            "user@example.com",
            "user@other.example"
        ));
    }

    #[test]
    fn expired_tokens_are_detected_strictly_after_the_instant() {
        let now = Utc::now();

        assert!(token_expired(now - Duration::seconds(1), now));
        assert!(!token_expired(now, now));
        assert!(!token_expired(now + Duration::seconds(1), now));
    }

    #[test]
    fn buckets_admit_requests_up_to_the_limit() {
        assert!(bucket_admits(1, 3));
        assert!(bucket_admits(3, 3));
        assert!(!bucket_admits(4, 3));
        assert!(!bucket_admits(1, 0));
        assert!(bucket_admits(1, 1));
        assert!(!bucket_admits(2, 1));
    }

    #[test]
    fn invitation_email_batches_must_be_valid_and_unique() {
        let ok = validate_invitation_emails(&[
            " User@Example.COM ".to_string(),
            "other@example.com".to_string(),
        ])
        .unwrap();

        assert_eq!(ok, vec!["user@example.com", "other@example.com"]);

        let duplicate =
            validate_invitation_emails(&["a@example.com".to_string(), "a@example.com".to_string()])
                .unwrap_err();

        assert_eq!(duplicate.code, "INVALID_INPUT");

        let malformed = validate_invitation_emails(&["not-an-email".to_string()]).unwrap_err();

        assert_eq!(malformed.code, "INVALID_INPUT");

        assert!(validate_invitation_emails(&[]).unwrap().is_empty());
    }

    #[test]
    fn reset_links_use_the_upstream_app_path() {
        assert_eq!(
            reset_password_link("http://localhost:3001/", "tok"),
            "http://localhost:3001/reset-password/tok"
        );
        assert_eq!(
            reset_password_link("http://localhost:3001", "tok"),
            "http://localhost:3001/reset-password/tok"
        );
    }

    #[test]
    fn verification_links_carry_the_token_and_email_params() {
        assert_eq!(
            verification_link("http://localhost:3001", "tok", "a@b.c"),
            "http://localhost:3001/verify-email?emailVerificationToken=tok&email=a@b.c"
        );
    }

    #[test]
    fn invitation_links_carry_the_upstream_search_params() {
        assert_eq!(
            invitation_link("http://localhost:3001", "tok", "a@b.c"),
            "http://localhost:3001/welcome?inviteToken=tok&email=a@b.c"
        );
    }

    #[test]
    fn reset_subjects_switch_between_reset_and_set() {
        assert_eq!(
            password_reset_subject(true),
            "Action Needed to Reset Password"
        );
        assert_eq!(
            password_reset_subject(false),
            "Action Needed to Set Password"
        );
    }

    #[test]
    fn subdomain_urls_keep_the_server_scheme_and_host() {
        let config = config_fixture();

        assert_eq!(
            subdomain_url(&config, Some("apple")),
            "http://apple.localhost:3000"
        );

        let mut https = config_fixture();
        https.server_url = "https://crm.nudge.dev".to_string();

        assert_eq!(
            subdomain_url(&https, Some("yc")),
            "https://yc.crm.nudge.dev"
        );
        assert_eq!(subdomain_url(&https, None), "https://crm.nudge.dev");
        assert_eq!(subdomain_url(&https, Some("")), "https://crm.nudge.dev");
    }

    #[test]
    fn cooldown_messages_report_the_remaining_time() {
        assert_eq!(
            cooldown_wait_message(45),
            "Please wait 45 seconds before requesting another verification email"
        );
        assert_eq!(
            cooldown_wait_message(90),
            "Please wait 1 minutes 30 seconds before requesting another verification email"
        );
    }

    #[test]
    fn same_work_domain_matches_only_equal_domains() {
        assert!(is_same_work_domain("a@twenty.com", "b@TWENTY.com"));
        assert!(!is_same_work_domain("a@twenty.com", "b@example.com"));
        assert!(!is_same_work_domain("no-domain", "b@twenty.com"));
        assert!(!is_same_work_domain("a@twenty.com", "no-domain"));
    }

    #[test]
    fn context_values_are_read_from_the_app_token_context() {
        let context = json!({ "email": "a@b.c", "roleId": Uuid::from_u128(7).to_string() });
        let role_id = Uuid::from_u128(7);

        assert_eq!(
            invitation_email_from_context(Some(&context)).as_deref(),
            Some("a@b.c")
        );
        assert_eq!(
            invitation_role_id_from_context(Some(&context)),
            Some(role_id)
        );
        assert_eq!(invitation_email_from_context(None), None);
        assert_eq!(invitation_role_id_from_context(None), None);
        assert_eq!(
            invitation_email_from_context(Some(&json!({ "roleId": "x" }))),
            None
        );
        assert_eq!(
            invitation_role_id_from_context(Some(&json!({ "roleId": "x" }))),
            None
        );
    }

    #[test]
    fn token_lifetimes_match_the_upstream_config_defaults() {
        assert_eq!(PASSWORD_RESET_EMAIL_RATE_LIMIT_MAX, 3);
        assert_eq!(PASSWORD_RESET_EMAIL_RATE_LIMIT_WINDOW_SECS, 15 * 60);
        assert_eq!(PASSWORD_RESET_TOKEN_EXPIRES_IN_SECS, 5 * 60);
        assert_eq!(EMAIL_VERIFICATION_TOKEN_EXPIRES_IN_SECS, 60 * 60);
        assert_eq!(INVITATION_TOKEN_EXPIRES_IN_SECS, 30 * 24 * 60 * 60);
        assert_eq!(EMAIL_VERIFICATION_RESEND_COOLDOWN_SECS, 60);
        assert_eq!(VERIFICATION_EMAIL_SENDING_LIMIT, 5);
        assert_eq!(VERIFICATION_EMAIL_SENDING_WINDOW_SECS, 86_400);
        assert_eq!(INVITATION_SENDING_BY_EMAIL_LIMIT, 10);
        assert_eq!(INVITATION_SENDING_BY_EMAIL_WINDOW_SECS, 7 * 24 * 60 * 60);
        assert_eq!(INVITATION_SENDING_BY_WORKSPACE_LIMIT, 500);
        assert_eq!(
            INVITATION_SENDING_BY_WORKSPACE_WINDOW_SECS,
            7 * 24 * 60 * 60
        );
        assert_eq!(INVITATION_SENDING_BY_SENDER_LIMIT, 20);
        assert_eq!(INVITATION_SENDING_BY_SENDER_SAME_WORK_DOMAIN_LIMIT, 50);
        assert_eq!(INVITATION_SENDING_BY_SENDER_WINDOW_SECS, 86_400);
    }

    #[test]
    fn throttle_keys_use_the_upstream_prefixes() {
        assert_eq!(
            format!("{PASSWORD_RESET_EMAIL_THROTTLE_PREFIX}a@b.c"),
            "password-reset-email:a@b.c"
        );
        assert_eq!(
            format!("{VERIFICATION_EMAIL_THROTTLE_PREFIX}a@b.c"),
            "verification-email-sending:throttler:a@b.c"
        );
        assert_eq!(
            format!("{INVITATION_SENDING_THROTTLE_PREFIX}ws"),
            "invitation-resending-workspace:throttler:ws"
        );
    }

    #[test]
    fn invitation_json_exposes_the_upstream_fields_without_the_token_value() {
        let expires_at = Utc::now();
        let role_id = Uuid::from_u128(9);
        let row = AppTokenRow {
            id: Uuid::from_u128(1),
            workspace_id: Some(Uuid::from_u128(2)),
            expires_at,
            value: "verbatim-secret".to_string(),
            r#type: APP_TOKEN_TYPE_INVITATION.to_string(),
            context: Some(json!({ "email": "a@b.c", "roleId": role_id.to_string() })),
        };

        let rendered = invitation_json(&row);

        assert_eq!(rendered["id"], Uuid::from_u128(1).to_string());
        assert_eq!(rendered["email"], "a@b.c");
        assert_eq!(rendered["roleId"], role_id.to_string());
        assert!(rendered.get("value").is_none());

        let plain = AppTokenRow {
            context: Some(json!({ "email": "a@b.c" })),
            ..row
        };

        assert_eq!(invitation_json(&plain)["roleId"], Value::Null);
    }

    #[test]
    fn email_verification_is_required_only_when_the_env_says_true() {
        std::env::remove_var("IS_EMAIL_VERIFICATION_REQUIRED");
        assert!(!is_email_verification_required());

        std::env::set_var("IS_EMAIL_VERIFICATION_REQUIRED", "true");
        assert!(is_email_verification_required());

        std::env::set_var("IS_EMAIL_VERIFICATION_REQUIRED", "TRUE");
        assert!(is_email_verification_required());

        std::env::set_var("IS_EMAIL_VERIFICATION_REQUIRED", "false");
        assert!(!is_email_verification_required());

        std::env::remove_var("IS_EMAIL_VERIFICATION_REQUIRED");
    }
}
