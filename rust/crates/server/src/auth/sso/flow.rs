use uuid::Uuid;

use super::super::error::{AuthError, AuthResult};
use super::super::tokens::{self, AuthProvider};
use super::state_store::{generate_plain_exchange_token, sha256_hex, SocialSsoState};
use crate::state::AppState;

pub const SSO_EXCHANGE_TOKEN_TTL_SECS: i64 = 300;
const INVITATION_TOKEN_TYPES: [&str; 2] = ["INVITATION_TOKEN", "ONBOARDING_INVITATION_TOKEN"];

#[derive(Debug, Clone)]
pub struct SocialProfile {
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceRow {
    pub id: Uuid,
    pub is_public_invite_link_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct InvitationRow {
    pub id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ExistingUserRow {
    pub id: Uuid,
    #[sqlx(rename = "passwordHash")]
    pub password_hash: Option<String>,
    #[sqlx(rename = "isEmailVerified")]
    pub is_email_verified: bool,
    #[sqlx(rename = "canImpersonate")]
    pub can_impersonate: bool,
    #[sqlx(rename = "canAccessFullAdminPanel")]
    pub can_access_full_admin_panel: bool,
}

pub fn frontend_base(config: &nudge_core::Config) -> String {
    config.frontend_url.trim_end_matches('/').to_string()
}

fn valid_return_to_path(raw: Option<&str>) -> Option<&str> {
    raw.filter(|raw| !raw.is_empty() && raw.starts_with('/'))
}

pub fn error_redirect_url(config: &nudge_core::Config, message: &str) -> String {
    format!(
        "{}/verify?errorMessage={}",
        frontend_base(config),
        super::percent_encode(message)
    )
}

pub fn welcome_redirect_url(
    config: &nudge_core::Config,
    exchange_token: &str,
    return_to_path: Option<&str>,
) -> String {
    let mut url = format!("{}/welcome", frontend_base(config));

    if let Some(path) = valid_return_to_path(return_to_path) {
        url.push_str(&format!("?returnToPath={}", super::percent_encode(path)));
    }

    url.push_str(&format!("#ssoExchangeToken={exchange_token}"));

    url
}

pub fn verify_redirect_url(
    config: &nudge_core::Config,
    login_token: &str,
    billing_checkout_session_state: Option<&str>,
    return_to_path: Option<&str>,
) -> String {
    let mut url = format!(
        "{}/verify?loginToken={}",
        frontend_base(config),
        super::percent_encode(login_token)
    );

    if let Some(billing) = billing_checkout_session_state.filter(|raw| !raw.is_empty()) {
        url.push_str(&format!(
            "&billingCheckoutSessionState={}",
            super::percent_encode(billing)
        ));
    }

    if let Some(path) = valid_return_to_path(return_to_path) {
        url.push_str(&format!("&returnToPath={}", super::percent_encode(path)));
    }

    url
}

pub fn idp_not_found_error() -> AuthError {
    AuthError::new(403, "OAUTH_ACCESS_DENIED", "Identity provider not found")
}

pub fn invalid_sso_exchange_token_error() -> AuthError {
    AuthError::new(400, "INVALID_INPUT", "Invalid SSO exchange token")
}

pub async fn issue_sso_exchange_token(
    state: &AppState,
    user_id: Uuid,
    auth_provider: AuthProvider,
) -> AuthResult<String> {
    let plain_token = generate_plain_exchange_token();

    sqlx::query(
        "INSERT INTO core.\"appToken\" (\"userId\", type, value, \"expiresAt\", context) \
         VALUES ($1, 'SSO_EXCHANGE_TOKEN', $2, now() + make_interval(secs => $3), $4)",
    )
    .bind(user_id)
    .bind(sha256_hex(&plain_token))
    .bind(SSO_EXCHANGE_TOKEN_TTL_SECS)
    .bind(serde_json::json!({ "authProvider": auth_provider.as_str() }).to_string())
    .execute(&state.core_pool)
    .await?;

    Ok(plain_token)
}

pub struct ConsumedSsoExchangeToken {
    pub user_id: Uuid,
    pub auth_provider: AuthProvider,
}

#[derive(Debug, sqlx::FromRow)]
struct SsoExchangeTokenRow {
    id: Uuid,
    #[sqlx(rename = "userId")]
    user_id: Option<Uuid>,
    #[sqlx(rename = "expiresAt")]
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    auth_provider: Option<String>,
}

pub async fn consume_sso_exchange_token(
    state: &AppState,
    plain_token: &str,
) -> AuthResult<ConsumedSsoExchangeToken> {
    let row: Option<SsoExchangeTokenRow> = sqlx::query_as(
        "SELECT id, \"userId\", \"expiresAt\", context->>'authProvider' AS auth_provider \
         FROM core.\"appToken\" \
         WHERE value = $1 AND type = 'SSO_EXCHANGE_TOKEN' \
           AND \"revokedAt\" IS NULL AND \"deletedAt\" IS NULL",
    )
    .bind(sha256_hex(plain_token))
    .fetch_optional(&state.core_pool)
    .await?;

    let row = row.ok_or_else(invalid_sso_exchange_token_error)?;

    let affected = sqlx::query(
        "DELETE FROM core.\"appToken\" \
         WHERE id = $1 AND \"revokedAt\" IS NULL AND \"deletedAt\" IS NULL",
    )
    .bind(row.id)
    .execute(&state.core_pool)
    .await?
    .rows_affected();

    if affected != 1 {
        return Err(invalid_sso_exchange_token_error());
    }

    let expires_at = row
        .expires_at
        .ok_or_else(invalid_sso_exchange_token_error)?;

    if chrono::Utc::now() > expires_at {
        return Err(invalid_sso_exchange_token_error());
    }

    let auth_provider = row
        .auth_provider
        .and_then(|raw| AuthProvider::parse(&raw))
        .ok_or_else(invalid_sso_exchange_token_error)?;

    let user_id = row.user_id.ok_or_else(invalid_sso_exchange_token_error)?;

    Ok(ConsumedSsoExchangeToken {
        user_id,
        auth_provider,
    })
}

pub async fn find_workspace_by_invite_hash(
    state: &AppState,
    invite_hash: &str,
) -> AuthResult<WorkspaceRow> {
    let row: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, \"isPublicInviteLinkEnabled\" \
         FROM core.\"workspace\" WHERE \"inviteHash\" = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(invite_hash)
    .fetch_optional(&state.core_pool)
    .await?;

    row.map(|(id, is_public_invite_link_enabled)| WorkspaceRow {
        id,
        is_public_invite_link_enabled,
    })
    .ok_or_else(|| AuthError::invalid_input("Workspace does not exist"))
}

pub async fn find_workspace_by_id(
    state: &AppState,
    workspace_id: &Uuid,
) -> AuthResult<Option<WorkspaceRow>> {
    let row: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, \"isPublicInviteLinkEnabled\" \
         FROM core.\"workspace\" WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row.map(|(id, is_public_invite_link_enabled)| WorkspaceRow {
        id,
        is_public_invite_link_enabled,
    }))
}

async fn find_invitation_row(
    state: &AppState,
    workspace_id: &Uuid,
    email: Option<&str>,
    personal_invite_token: Option<&str>,
) -> AuthResult<Option<InvitationRow>> {
    let mut query = format!(
        "SELECT id FROM core.\"appToken\" \
         WHERE \"workspaceId\" = $1 AND type IN ('{}', '{}') \
           AND \"deletedAt\" IS NULL AND \"expiresAt\" > now()",
        INVITATION_TOKEN_TYPES[0], INVITATION_TOKEN_TYPES[1]
    );

    if email.is_some() {
        query.push_str(" AND lower(context->>'email') = lower($2)");
    }

    if personal_invite_token.is_some() {
        query.push_str(if email.is_some() {
            " AND value = $3"
        } else {
            " AND value = $2"
        });
    }

    let mut statement = sqlx::query_as::<_, (Uuid,)>(&query).bind(workspace_id);

    if let Some(email) = email {
        statement = statement.bind(email);
    }

    if let Some(token) = personal_invite_token {
        statement = statement.bind(token);
    }

    let row = statement.fetch_optional(&state.core_pool).await?;

    Ok(row.map(|(id,)| InvitationRow { id }))
}

pub async fn find_invitation_by_email(
    state: &AppState,
    workspace_id: &Uuid,
    email: &str,
) -> AuthResult<Option<InvitationRow>> {
    find_invitation_row(state, workspace_id, Some(email), None).await
}

pub async fn find_invitation_by_token_and_email(
    state: &AppState,
    workspace_id: &Uuid,
    personal_invite_token: &str,
    email: &str,
) -> AuthResult<Option<InvitationRow>> {
    find_invitation_row(
        state,
        workspace_id,
        Some(email),
        Some(personal_invite_token),
    )
    .await
}

pub async fn find_existing_user(
    state: &AppState,
    email: &str,
) -> AuthResult<Option<ExistingUserRow>> {
    let row: Option<ExistingUserRow> = sqlx::query_as(
        "SELECT id, \"passwordHash\", \"isEmailVerified\", \"canImpersonate\", \"canAccessFullAdminPanel\" \
         FROM core.\"user\" WHERE email = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(email)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

pub async fn user_has_workspace_membership(
    state: &AppState,
    user_id: &Uuid,
    workspace_id: &Uuid,
) -> AuthResult<bool> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row.is_some())
}

async fn has_server_admin(state: &AppState) -> AuthResult<bool> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT count(*) FROM core.\"user\" \
         WHERE \"canAccessFullAdminPanel\" = true AND \"deletedAt\" IS NULL",
    )
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row.map(|(count,)| count).unwrap_or(0) > 0)
}

pub fn email_domain(email: &str) -> Option<&str> {
    email
        .rsplit_once('@')
        .map(|(_, domain)| domain)
        .filter(|domain| !domain.is_empty())
}

pub async fn email_matches_approved_access_domain(
    state: &AppState,
    workspace_id: &Uuid,
    email: &str,
) -> AuthResult<bool> {
    if !super::config::is_email_verification_required() {
        return Ok(false);
    }

    let Some(domain) = email_domain(email) else {
        return Ok(false);
    };

    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"approvedAccessDomain\" \
         WHERE \"workspaceId\" = $1 AND \"isValidated\" = true AND domain = $2 LIMIT 1",
    )
    .bind(workspace_id)
    .bind(domain)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row.is_some())
}

async fn has_provisioned_destination(state: &AppState, email: &str) -> AuthResult<bool> {
    let invitation: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"appToken\" \
         WHERE type IN ('INVITATION_TOKEN', 'ONBOARDING_INVITATION_TOKEN') \
           AND \"deletedAt\" IS NULL AND \"expiresAt\" > now() \
           AND lower(context->>'email') = lower($1) LIMIT 1",
    )
    .bind(email)
    .fetch_optional(&state.core_pool)
    .await?;

    if invitation.is_some() {
        return Ok(true);
    }

    let membership: Option<(Uuid,)> = sqlx::query_as(
        "SELECT uw.id FROM core.\"userWorkspace\" uw \
         JOIN core.\"workspace\" w ON w.id = uw.\"workspaceId\" \
         JOIN core.\"user\" u ON u.id = uw.\"userId\" \
         WHERE lower(u.email) = lower($1) \
           AND w.\"deletedAt\" IS NULL AND uw.\"deletedAt\" IS NULL \
           AND w.\"activationStatus\" IN ('CREATED', 'ACTIVE', 'SUSPENDED') LIMIT 1",
    )
    .bind(email)
    .fetch_optional(&state.core_pool)
    .await?;

    if membership.is_some() {
        return Ok(true);
    }

    if super::config::is_email_verification_required() {
        if let Some(domain) = email_domain(email) {
            let approved: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM core.\"approvedAccessDomain\" \
                 WHERE \"isValidated\" = true AND domain = $1 LIMIT 1",
            )
            .bind(domain)
            .fetch_optional(&state.core_pool)
            .await?;

            if approved.is_some() {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

async fn assert_sign_up_without_workspace_allowed(state: &AppState, email: &str) -> AuthResult<()> {
    let workspace_count: Option<(i64,)> = sqlx::query_as("SELECT count(*) FROM core.\"workspace\"")
        .fetch_optional(&state.core_pool)
        .await?;

    let workspace_count = workspace_count.map(|(count,)| count).unwrap_or(0);

    let allowed = workspace_count == 0
        || (super::config::is_multiworkspace_enabled()
            && !super::config::is_workspace_creation_limited_to_server_admins());

    if allowed {
        return Ok(());
    }

    if has_provisioned_destination(state, email).await? {
        return Ok(());
    }

    Err(AuthError::new(
        401,
        "SIGNUP_DISABLED",
        "New workspace setup is disabled",
    ))
}

async fn create_user(
    state: &AppState,
    email: &str,
    profile: &SocialProfile,
    locale: &str,
    grant_server_admin: bool,
) -> AuthResult<Uuid> {
    let user_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO core.\"user\" \
         (email, \"isEmailVerified\", \"firstName\", \"lastName\", locale, \
          \"canImpersonate\", \"canAccessFullAdminPanel\") \
         VALUES ($1, true, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(email)
    .bind(profile.first_name.as_deref().unwrap_or(""))
    .bind(profile.last_name.as_deref().unwrap_or(""))
    .bind(locale)
    .bind(grant_server_admin)
    .bind(grant_server_admin)
    .fetch_one(&state.core_pool)
    .await?;

    Ok(user_id.0)
}

async fn insert_user_workspace(
    state: &AppState,
    user_id: &Uuid,
    workspace_id: &Uuid,
) -> AuthResult<()> {
    sqlx::query(
        "INSERT INTO core.\"userWorkspace\" (\"userId\", \"workspaceId\", locale) \
         VALUES ($1, $2, 'en')",
    )
    .bind(user_id)
    .bind(workspace_id)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

async fn invalidate_invitation(state: &AppState, invitation_id: &Uuid) -> AuthResult<()> {
    sqlx::query(
        "UPDATE core.\"appToken\" SET \"deletedAt\" = now(), \"updatedAt\" = now() WHERE id = $1",
    )
    .bind(invitation_id)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

pub async fn sign_in_up_with_social_sso(
    state: &AppState,
    profile: SocialProfile,
    social_state: &SocialSsoState,
    auth_provider: AuthProvider,
) -> AuthResult<String> {
    let email = profile.email.to_lowercase();

    let existing_user = find_existing_user(state, &email).await?;

    let social_workspace_id = social_state
        .workspace_id
        .as_deref()
        .and_then(|raw| Uuid::parse_str(raw).ok());

    if social_workspace_id.is_none() && social_state.workspace_invite_hash.is_none() {
        let user_id = match existing_user {
            Some(existing) => existing.id,
            None => {
                assert_sign_up_without_workspace_allowed(state, &email).await?;

                let grant_server_admin = !has_server_admin(state).await?;

                let locale = social_state
                    .locale
                    .clone()
                    .filter(|raw| !raw.is_empty())
                    .unwrap_or_else(|| "en".to_string());

                create_user(state, &email, &profile, &locale, grant_server_admin).await?
            }
        };

        let exchange_token = issue_sso_exchange_token(state, user_id, auth_provider).await?;

        return Ok(welcome_redirect_url(
            &state.config,
            &exchange_token,
            social_state.return_to_path.as_deref(),
        ));
    }

    let workspace = resolve_workspace_for_sign_in_up(
        state,
        social_workspace_id,
        social_state.workspace_invite_hash.as_deref(),
    )
    .await?;

    match complete_workspace_sign_in_up(
        state,
        workspace.as_ref(),
        &email,
        &profile,
        social_state.workspace_invite_hash.as_deref(),
        None,
        social_state.billing_checkout_session_state.as_deref(),
        social_state.return_to_path.as_deref(),
        auth_provider,
        false,
    )
    .await
    {
        Ok(redirect) => Ok(redirect),
        Err(error) => Ok(error_redirect_url(
            &state.config,
            &error.messages.join(", "),
        )),
    }
}

async fn resolve_workspace_for_sign_in_up(
    state: &AppState,
    workspace_id: Option<Uuid>,
    invite_hash: Option<&str>,
) -> AuthResult<Option<WorkspaceRow>> {
    match invite_hash {
        Some(invite_hash) if !invite_hash.is_empty() => Ok(Some(
            find_workspace_by_invite_hash(state, invite_hash).await?,
        )),
        _ => {
            let Some(workspace_id) = workspace_id else {
                return Err(AuthError::workspace_not_found(
                    "A workspace id is required to find the sign-in workspace",
                ));
            };

            find_workspace_by_id(state, &workspace_id).await
        }
    }
}

struct SsoCallbackRequest<'a> {
    identity_provider_id: &'a str,
    email: &'a str,
    workspace_invite_hash: Option<&'a str>,
    workspace_personal_invite_token: Option<&'a str>,
}

pub async fn sso_idp_callback_redirect_url(
    state: &AppState,
    identity_provider_id: &str,
    email: &str,
    workspace_invite_hash: Option<&str>,
    workspace_personal_invite_token: Option<&str>,
) -> AuthResult<String> {
    let request = SsoCallbackRequest {
        identity_provider_id,
        email,
        workspace_invite_hash,
        workspace_personal_invite_token,
    };

    let result = sso_idp_sign_in_up(state, &request).await;

    match result {
        Ok(redirect) => Ok(redirect),
        Err(error) => Ok(error_redirect_url(
            &state.config,
            &error.messages.join(", "),
        )),
    }
}

async fn sso_idp_sign_in_up(
    state: &AppState,
    request: &SsoCallbackRequest<'_>,
) -> AuthResult<String> {
    let idp_id =
        Uuid::parse_str(request.identity_provider_id).map_err(|_| idp_not_found_error())?;

    let idp_row: Option<(Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT id, \"workspaceId\", status FROM core.\"workspaceSSOIdentityProvider\" WHERE id = $1",
    )
    .bind(idp_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (_, idp_workspace_id, status) = idp_row.ok_or_else(idp_not_found_error)?;

    if status != "Active" {
        return Err(idp_not_found_error());
    }

    let email = request.email.to_lowercase();

    let workspace = match request.workspace_invite_hash {
        Some(invite_hash) if !invite_hash.is_empty() => {
            find_workspace_by_invite_hash(state, invite_hash).await?
        }
        _ => find_workspace_by_id(state, &idp_workspace_id)
            .await?
            .ok_or_else(idp_not_found_error)?,
    };

    if workspace.id != idp_workspace_id {
        return Err(AuthError::new(
            403,
            "OAUTH_ACCESS_DENIED",
            "Identity provider does not belong to this workspace",
        ));
    }

    let existing_user = find_existing_user(state, &email).await?;

    let invitation = match (&existing_user, request.workspace_personal_invite_token) {
        (Some(_), Some(token)) if !token.is_empty() => {
            find_invitation_by_token_and_email(state, &workspace.id, token, &email).await?
        }
        (None, _) => find_invitation_by_email(state, &workspace.id, &email).await?,
        _ => None,
    };

    complete_workspace_sign_in_up(
        state,
        Some(&workspace),
        &email,
        &SocialProfile {
            email: email.clone(),
            first_name: None,
            last_name: None,
        },
        request.workspace_invite_hash,
        invitation.as_ref(),
        None,
        None,
        AuthProvider::Sso,
        true,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn complete_workspace_sign_in_up(
    state: &AppState,
    workspace: Option<&WorkspaceRow>,
    email: &str,
    profile: &SocialProfile,
    workspace_invite_hash: Option<&str>,
    idp_invitation: Option<&InvitationRow>,
    billing_checkout_session_state: Option<&str>,
    return_to_path: Option<&str>,
    auth_provider: AuthProvider,
    idp_guards: bool,
) -> AuthResult<String> {
    let Some(workspace) = workspace else {
        return Err(AuthError::forbidden(
            "User does not have access to this workspace",
        ));
    };

    let existing_user = find_existing_user(state, email).await?;

    let invitation = match idp_invitation {
        Some(invitation) => Some(invitation.clone()),
        None if idp_guards => None,
        None => find_invitation_by_email(state, &workspace.id, email).await?,
    };

    let has_personal_invitation = invitation.is_some();
    let has_public_invite_link = workspace_invite_hash
        .map(|hash| !hash.is_empty())
        .unwrap_or(false);

    let check_access_applies = !idp_guards || existing_user.is_none();

    let approved_domain_grant = if check_access_applies {
        email_matches_approved_access_domain(state, &workspace.id, email).await?
    } else {
        false
    };

    if !approved_domain_grant {
        if has_public_invite_link
            && !has_personal_invitation
            && !workspace.is_public_invite_link_enabled
        {
            return Err(AuthError::forbidden(
                "Public invite link is disabled for this workspace",
            ));
        }

        let is_invited_to_workspace = if idp_guards && existing_user.is_some() {
            has_personal_invitation
        } else {
            has_personal_invitation || has_public_invite_link
        };

        match existing_user.as_ref() {
            Some(existing) => {
                if idp_guards {
                    if invitation.is_some()
                        && (existing.can_access_full_admin_panel || existing.can_impersonate)
                    {
                        return Err(AuthError::forbidden(
                            "Accounts with admin panel or impersonation access cannot join a workspace through SSO. Accept the invitation with another sign-in method.",
                        ));
                    }

                    if invitation.is_some()
                        && existing
                            .password_hash
                            .as_deref()
                            .is_some_and(|hash| !hash.is_empty())
                        && !existing.is_email_verified
                    {
                        return Err(AuthError::forbidden(
                            "An unverified account already exists for this email. Verify its email or reset its password before accepting the invitation through SSO.",
                        ));
                    }
                }

                if !is_invited_to_workspace
                    && !user_has_workspace_membership(state, &existing.id, &workspace.id).await?
                {
                    return Err(AuthError::forbidden(
                        "User does not have access to this workspace",
                    ));
                }
            }
            None => {
                if !is_invited_to_workspace {
                    return Err(AuthError::forbidden(
                        "User does not have access to this workspace",
                    ));
                }
            }
        }
    }

    let _user_id = match existing_user {
        Some(existing) => {
            let is_member =
                user_has_workspace_membership(state, &existing.id, &workspace.id).await?;

            if invitation.is_some() && !is_member {
                insert_user_workspace(state, &existing.id, &workspace.id).await?;

                if let Some(invitation_row) = invitation.as_ref() {
                    invalidate_invitation(state, &invitation_row.id).await?;
                }
            }

            if idp_guards
                && !existing.is_email_verified
                && existing
                    .password_hash
                    .as_deref()
                    .is_none_or(|hash| hash.is_empty())
            {
                sqlx::query(
                    "UPDATE core.\"user\" SET \"isEmailVerified\" = true, \"updatedAt\" = now() \
                     WHERE id = $1",
                )
                .bind(existing.id)
                .execute(&state.core_pool)
                .await?;
            }

            existing.id
        }
        None => {
            let user_id = create_user(state, email, profile, "en", false).await?;

            insert_user_workspace(state, &user_id, &workspace.id).await?;

            if let Some(invitation) = invitation.as_ref() {
                invalidate_invitation(state, &invitation.id).await?;
            }

            user_id
        }
    };

    let login_token = tokens::issue_login_token(state, email, workspace.id, auth_provider).await?;

    Ok(verify_redirect_url(
        &state.config,
        &login_token,
        billing_checkout_session_state,
        return_to_path,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> nudge_core::Config {
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
        }
    }

    #[test]
    fn welcome_redirect_keeps_the_token_in_the_fragment() {
        let settings = config();

        assert_eq!(
            welcome_redirect_url(&settings, "tok123", None),
            "http://localhost:3001/welcome#ssoExchangeToken=tok123"
        );

        assert_eq!(
            welcome_redirect_url(&settings, "tok123", Some("/settings")),
            "http://localhost:3001/welcome?returnToPath=%2Fsettings#ssoExchangeToken=tok123"
        );
    }

    #[test]
    fn return_to_paths_outside_the_site_are_dropped() {
        let settings = config();

        assert_eq!(
            welcome_redirect_url(&settings, "tok", Some("https://evil.example")),
            "http://localhost:3001/welcome#ssoExchangeToken=tok"
        );

        assert_eq!(
            welcome_redirect_url(&settings, "tok", Some("")),
            "http://localhost:3001/welcome#ssoExchangeToken=tok"
        );

        assert_eq!(
            welcome_redirect_url(&settings, "tok", Some("relative-no-slash")),
            "http://localhost:3001/welcome#ssoExchangeToken=tok"
        );
    }

    #[test]
    fn verify_redirect_carries_login_token_billing_and_return_path() {
        let settings = config();

        assert_eq!(
            verify_redirect_url(&settings, "ltok", None, None),
            "http://localhost:3001/verify?loginToken=ltok"
        );

        assert_eq!(
            verify_redirect_url(&settings, "ltok", Some("cs_abc"), Some("/profile")),
            "http://localhost:3001/verify?loginToken=ltok&billingCheckoutSessionState=cs_abc&returnToPath=%2Fprofile"
        );
    }

    #[test]
    fn error_redirects_encode_the_message_for_the_frontend() {
        let settings = config();

        assert_eq!(
            error_redirect_url(&settings, "Google auth is not enabled"),
            "http://localhost:3001/verify?errorMessage=Google%20auth%20is%20not%20enabled"
        );
    }

    #[test]
    fn invalid_sso_exchange_tokens_are_a_single_opaque_error() {
        let error = invalid_sso_exchange_token_error();

        assert_eq!(error.status, 400);
        assert_eq!(error.code, "INVALID_INPUT");
        assert_eq!(error.messages.join(", "), "Invalid SSO exchange token");
    }

    #[test]
    fn idp_errors_use_the_upstream_oauth_access_denied_code() {
        let error = idp_not_found_error();

        assert_eq!(error.status, 403);
        assert_eq!(error.code, "OAUTH_ACCESS_DENIED");
    }

    #[test]
    fn email_domains_split_at_the_last_at_sign() {
        assert_eq!(email_domain("jane@example.co.uk"), Some("example.co.uk"));
        assert_eq!(email_domain("no-at-sign"), None);
        assert_eq!(email_domain("trailing@"), None);
    }
}
