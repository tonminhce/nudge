use axum::extract::{ConnectInfo, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use nudge_core::ids::workspace_schema_name;
use nudge_core::NodeEnv;
use rand::Rng;
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use uuid::Uuid;

use super::error::{AuthError, AuthResult};
use super::jwt_service::TokenType;
use super::middleware::AuthContext;
use super::service::{
    LoginTokenInput, ADMIN_ROLE_UNIVERSAL_IDENTIFIER, MEMBER_ROLE_UNIVERSAL_IDENTIFIER,
};
use super::session::{self, CreationOrigin, RevocationReason, SessionRequestContext};
use super::tokens::{self, AuthProvider, IssuedTokenPair};
use crate::state::AppState;

const WORKSPACE_CUSTOM_APPLICATION_ROLE_UNIVERSAL_IDENTIFIER: &str =
    "b5b505e2-8834-457f-a301-f1f4694fe365";
const WORKSPACE_CUSTOM_APPLICATION_ROLE_LABEL: &str = "Workspace Custom Application";
const IMPERSONATION_REFRESH_LIFETIME_SECS: i64 = 24 * 60 * 60;
const WORKSPACE_ACTIVATION_STALE_LOCK_SECS: i64 = 5 * 60;
const SUBDOMAIN_MAX_LENGTH: usize = 30;
const MAX_NUMBERED_SUFFIX_ATTEMPTS: i64 = 50;
const MAX_RANDOM_FALLBACK_ATTEMPTS: usize = 10;

const PERMISSION_DENIED_MESSAGE: &str = "Entity performing the request does not have permission";
const IMPERSONATE_VIA_API_KEY_MESSAGE: &str = "Can't impersonate user via api key";
const IMPERSONATE_PERMISSION_MESSAGE: &str =
    "You do not have permission to impersonate users. Please contact your workspace administrator for access.";
const SELF_IMPERSONATION_MESSAGE: &str = "User cannot impersonate themselves";
const SERVER_LEVEL_NOT_ALLOWED_MESSAGE: &str = "Server level impersonation not allowed";
const WORKSPACE_LEVEL_NOT_ALLOWED_MESSAGE: &str = "Impersonation not allowed";
const TARGET_ADMIN_PRIVILEGES_MESSAGE: &str =
    "Cannot impersonate a user with admin privileges. Only administrators can impersonate other administrators.";
const NOT_IMPERSONATING_MESSAGE: &str = "Not currently impersonating";
const TWO_FACTOR_PROVISION_MESSAGE: &str =
    "Two-factor authentication is required for server-level impersonation. Please enable 2FA in your workspace settings before attempting to impersonate users.";
const TWO_FACTOR_VERIFICATION_MESSAGE: &str =
    "Two-factor authentication is required for server-level impersonation. Please verify your 2FA method before attempting to impersonate users.";

const SUBDOMAIN_RANDOM_PREFIXES: [&str; 55] = [
    "bold",
    "bouncy",
    "brave",
    "bright",
    "calm",
    "clever",
    "colorful",
    "cool",
    "eager",
    "fast",
    "fierce",
    "funny",
    "gentle",
    "glad",
    "graceful",
    "happy",
    "joyful",
    "kind",
    "lucky",
    "mighty",
    "noble",
    "quick",
    "sharp",
    "shiny",
    "silent",
    "smart",
    "swift",
    "wise",
    "witty",
    "zany",
    "agile",
    "amazing",
    "awesome",
    "brilliant",
    "cheerful",
    "dashing",
    "dynamic",
    "elegant",
    "energetic",
    "excellent",
    "fantastic",
    "friendly",
    "glorious",
    "grand",
    "helpful",
    "honest",
    "innovative",
    "inspiring",
    "jolly",
    "lively",
    "magical",
    "optimistic",
    "peaceful",
    "powerful",
    "radiant",
];

const SUBDOMAIN_RANDOM_COLORS: [&str; 48] = [
    "almond",
    "amber",
    "azure",
    "beige",
    "black",
    "blue",
    "brown",
    "crimson",
    "cyan",
    "gold",
    "gray",
    "green",
    "indigo",
    "lime",
    "magenta",
    "maroon",
    "mint",
    "mustard",
    "orange",
    "peach",
    "pink",
    "purple",
    "red",
    "salmon",
    "silver",
    "teal",
    "turquoise",
    "violet",
    "white",
    "yellow",
    "aqua",
    "bronze",
    "burgundy",
    "cerulean",
    "charcoal",
    "coral",
    "emerald",
    "fuchsia",
    "garnet",
    "jade",
    "lavender",
    "navy",
    "platinum",
    "plum",
    "rose",
    "ruby",
    "sapphire",
    "scarlet",
];

const SUBDOMAIN_RANDOM_SUFFIXES: [&str; 50] = [
    "bear",
    "crocodile",
    "dolphin",
    "eagle",
    "elephant",
    "falcon",
    "fox",
    "giraffe",
    "hawk",
    "hedgehog",
    "horse",
    "kangaroo",
    "koala",
    "lion",
    "lynx",
    "monkey",
    "moose",
    "octopus",
    "owl",
    "panda",
    "panther",
    "penguin",
    "rabbit",
    "raccoon",
    "seal",
    "shark",
    "sparrow",
    "tiger",
    "whale",
    "wolf",
    "antelope",
    "badger",
    "buffalo",
    "camel",
    "cheetah",
    "deer",
    "dragon",
    "gazelle",
    "gorilla",
    "hippo",
    "jaguar",
    "lemur",
    "leopard",
    "llama",
    "meerkat",
    "otter",
    "peacock",
    "phoenix",
    "rhino",
    "zebra",
];

const RESERVED_SUBDOMAINS: [&str; 122] = [
    "about",
    "admin",
    "africa",
    "al",
    "america",
    "api",
    "app",
    "asia",
    "assets",
    "au",
    "audio",
    "auth",
    "az",
    "ba",
    "be",
    "bg",
    "billing",
    "blog",
    "ca",
    "careers",
    "cdn",
    "ch",
    "chat",
    "community",
    "companies",
    "config",
    "contact",
    "cy",
    "cz",
    "dash",
    "dashboard",
    "db",
    "de",
    "demo",
    "dev",
    "developer",
    "dk",
    "docs",
    "ee",
    "es",
    "eu",
    "europe",
    "events",
    "favicon",
    "feedback",
    "fi",
    "files",
    "forum",
    "fr",
    "gr",
    "help",
    "hr",
    "hu",
    "image",
    "images",
    "inbound",
    "is",
    "it",
    "jobs",
    "legal",
    "login",
    "logs",
    "london",
    "lt",
    "lv",
    "mail",
    "main",
    "me",
    "media",
    "merch",
    "metrics",
    "mk",
    "new-york",
    "news",
    "next",
    "nl",
    "no",
    "north-africa",
    "north-america",
    "nz",
    "oceania",
    "otel-collector",
    "paris",
    "partners",
    "partnership",
    "partnerships",
    "payment",
    "pl",
    "privacy",
    "production",
    "pt",
    "register",
    "ro",
    "rs",
    "ru",
    "san-francisco",
    "se",
    "settings",
    "shop",
    "si",
    "signin",
    "signup",
    "sk",
    "south-africa",
    "south-america",
    "staging",
    "storage",
    "store",
    "support",
    "t",
    "telemetry",
    "terms",
    "test",
    "testing",
    "tr",
    "trust",
    "ua",
    "uk",
    "us",
    "video",
    "www",
    "za",
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub subdomain: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpersonateInput {
    pub workspace_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImpersonationLevel {
    Server,
    Workspace,
}

fn impersonation_level(
    impersonator_workspace_id: Uuid,
    target_workspace_id: Uuid,
) -> ImpersonationLevel {
    if target_workspace_id == impersonator_workspace_id {
        ImpersonationLevel::Workspace
    } else {
        ImpersonationLevel::Server
    }
}

struct ImpersonatorParty {
    user_workspace_id: Uuid,
    user_id: Uuid,
    workspace_id: Uuid,
    can_impersonate: bool,
    is_full_admin: bool,
}

struct TargetParty {
    user_workspace_id: Uuid,
    user_id: Uuid,
    workspace_id: Uuid,
    email: String,
    allow_impersonation: bool,
    can_impersonate: bool,
    is_full_admin: bool,
}

fn user_has_admin_privileges(can_impersonate: bool, is_full_admin: bool) -> bool {
    can_impersonate || is_full_admin
}

fn user_can_server_impersonate_target(
    impersonator_is_full_admin: bool,
    target_is_full_admin: bool,
) -> bool {
    !target_is_full_admin || impersonator_is_full_admin
}

fn user_can_workspace_impersonate_target(impersonator: (bool, bool), target: (bool, bool)) -> bool {
    (!user_has_admin_privileges(target.0, target.1)
        || user_has_admin_privileges(impersonator.0, impersonator.1))
        && (!target.1 || impersonator.1)
}

fn is_server_admin_access_allowed(
    impersonator_workspace_id: Uuid,
    server_admin_workspace_id: Option<&str>,
) -> bool {
    match server_admin_workspace_id {
        None => true,
        Some(server_admin_workspace_id) => {
            impersonator_workspace_id.to_string() == server_admin_workspace_id.to_lowercase()
        }
    }
}

fn is_valid_subdomain(subdomain: &str) -> bool {
    let valid_shape = !subdomain.is_empty()
        && subdomain.len() <= SUBDOMAIN_MAX_LENGTH
        && subdomain
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && subdomain
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && subdomain
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && !subdomain.starts_with("api-");

    valid_shape && !RESERVED_SUBDOMAINS.contains(&subdomain)
}

fn subdomain_slug_from_display_name(display_name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut last_was_separator = true;

    for character in display_name.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            slug.push('-');
            last_was_separator = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    let slug = slug.chars().take(SUBDOMAIN_MAX_LENGTH).collect::<String>();
    let trimmed = slug.trim_end_matches('-').to_string();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn subdomain_from_email(email: &str) -> Option<String> {
    let domain = email.split('@').nth(1)?;

    let label = domain.split('.').next()?;

    if label.is_empty() {
        None
    } else {
        Some(label.to_ascii_lowercase())
    }
}

fn generate_random_subdomain() -> String {
    let mut rng = rand::thread_rng();

    let prefix = SUBDOMAIN_RANDOM_PREFIXES[rng.gen_range(0..SUBDOMAIN_RANDOM_PREFIXES.len())];
    let color = SUBDOMAIN_RANDOM_COLORS[rng.gen_range(0..SUBDOMAIN_RANDOM_COLORS.len())];
    let suffix = SUBDOMAIN_RANDOM_SUFFIXES[rng.gen_range(0..SUBDOMAIN_RANDOM_SUFFIXES.len())];

    format!("{prefix}-{color}-{suffix}")
}

fn permission_denied(message: &str) -> AuthError {
    AuthError::new(403, "PERMISSION_DENIED", message)
}

fn user_workspace_not_found_error(message: &'static str) -> AuthError {
    AuthError::new(500, "USER_WORKSPACE_NOT_FOUND", message)
}

fn subdomain_not_valid_error() -> AuthError {
    AuthError::new(400, "SUBDOMAIN_NOT_VALID", "Subdomain is not valid")
}

fn subdomain_already_taken_error() -> AuthError {
    AuthError::new(400, "SUBDOMAIN_ALREADY_TAKEN", "Subdomain already taken")
}

fn server_level_not_allowed_error() -> AuthError {
    AuthError::forbidden(SERVER_LEVEL_NOT_ALLOWED_MESSAGE)
}

fn workspace_level_not_allowed_error() -> AuthError {
    AuthError::forbidden(WORKSPACE_LEVEL_NOT_ALLOWED_MESSAGE)
}

fn target_admin_privileges_error() -> AuthError {
    AuthError::forbidden(TARGET_ADMIN_PRIVILEGES_MESSAGE)
}

fn impersonation_refresh_lifetime_secs() -> i64 {
    IMPERSONATION_REFRESH_LIFETIME_SECS
}

fn impersonation_access_claims(
    user_id: Uuid,
    workspace_id: Uuid,
    user_workspace_id: Uuid,
    impersonator_user_workspace_id: Uuid,
    impersonated_user_workspace_id: Uuid,
) -> Value {
    json!({
        "sub": user_id,
        "userId": user_id,
        "workspaceId": workspace_id,
        "userWorkspaceId": user_workspace_id,
        "authProvider": AuthProvider::Impersonation.as_str(),
        "isImpersonating": true,
        "impersonatorUserWorkspaceId": impersonator_user_workspace_id,
        "impersonatedUserWorkspaceId": impersonated_user_workspace_id,
    })
}

fn impersonation_refresh_claims(
    user_id: Uuid,
    workspace_id: Uuid,
    impersonator_user_workspace_id: Uuid,
    impersonated_user_workspace_id: Uuid,
) -> Value {
    json!({
        "sub": user_id,
        "userId": user_id,
        "workspaceId": workspace_id,
        "authProvider": AuthProvider::Impersonation.as_str(),
        "targetedTokenType": TokenType::Access.as_str(),
        "isImpersonating": true,
        "impersonatorUserWorkspaceId": impersonator_user_workspace_id,
        "impersonatedUserWorkspaceId": impersonated_user_workspace_id,
    })
}

fn can_restore_impersonator_session(payload: &Value, impersonator_user_workspace_id: Uuid) -> bool {
    let payload_type = payload.get("type").and_then(Value::as_str);

    let not_impersonating = !payload
        .get("isImpersonating")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    payload_type == Some(TokenType::Access.as_str())
        && not_impersonating
        && payload
            .get("userWorkspaceId")
            .and_then(Value::as_str)
            .and_then(|raw| Uuid::parse_str(raw).ok())
            == Some(impersonator_user_workspace_id)
}

async fn user_workspace_has_permission_flag(
    state: &AppState,
    workspace_id: Uuid,
    user_workspace_id: Uuid,
) -> AuthResult<bool> {
    let granted: (bool,) = sqlx::query_as(
        "SELECT EXISTS ( \
             SELECT 1 FROM core.\"roleTarget\" rt \
             JOIN core.role r ON r.id = rt.\"roleId\" \
             LEFT JOIN core.\"rolePermissionFlag\" rpf ON rpf.\"roleId\" = r.id \
             LEFT JOIN core.\"permissionFlag\" pf ON pf.id = rpf.\"permissionFlagId\" \
             WHERE rt.\"workspaceId\" = $1 AND rt.\"userWorkspaceId\" = $2 \
               AND (r.\"canUpdateAllSettings\" OR pf.key = 'IMPERSONATE') \
         )",
    )
    .bind(workspace_id)
    .bind(user_workspace_id)
    .fetch_one(&state.core_pool)
    .await?;

    Ok(granted.0)
}

async fn two_factor_counts(state: &AppState, user_workspace_id: Uuid) -> AuthResult<(i64, i64)> {
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE status = 'VERIFIED') \
         FROM core.\"twoFactorAuthenticationMethod\" \
         WHERE \"userWorkspaceId\" = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(user_workspace_id)
    .fetch_one(&state.core_pool)
    .await?;

    Ok(counts)
}

async fn check_impersonation_authorization(
    state: &AppState,
    impersonator: &ImpersonatorParty,
    target: &TargetParty,
) -> AuthResult<ImpersonationLevel> {
    match impersonation_level(impersonator.workspace_id, target.workspace_id) {
        ImpersonationLevel::Server => {
            let server_admin_gate = is_server_admin_access_allowed(impersonator.workspace_id, None);

            if !(impersonator.can_impersonate && target.allow_impersonation && server_admin_gate) {
                return Err(server_level_not_allowed_error());
            }

            if state.config.node_env != NodeEnv::Development {
                let (defined, verified) =
                    two_factor_counts(state, impersonator.user_workspace_id).await?;

                if defined == 0 {
                    return Err(AuthError::new(
                        401,
                        "TWO_FACTOR_AUTHENTICATION_PROVISION_REQUIRED",
                        TWO_FACTOR_PROVISION_MESSAGE,
                    ));
                }

                if verified == 0 {
                    return Err(AuthError::new(
                        401,
                        "TWO_FACTOR_AUTHENTICATION_VERIFICATION_REQUIRED",
                        TWO_FACTOR_VERIFICATION_MESSAGE,
                    ));
                }
            }

            if !user_can_server_impersonate_target(impersonator.is_full_admin, target.is_full_admin)
            {
                return Err(target_admin_privileges_error());
            }
        }
        ImpersonationLevel::Workspace => {
            let has_flag = user_workspace_has_permission_flag(
                state,
                target.workspace_id,
                impersonator.user_workspace_id,
            )
            .await?;

            if !has_flag {
                return Err(workspace_level_not_allowed_error());
            }

            if !user_can_workspace_impersonate_target(
                (impersonator.can_impersonate, impersonator.is_full_admin),
                (target.can_impersonate, target.is_full_admin),
            ) {
                return Err(target_admin_privileges_error());
            }
        }
    }

    Ok(impersonation_level(
        impersonator.workspace_id,
        target.workspace_id,
    ))
}

async fn load_impersonator_party(
    state: &AppState,
    impersonator_user_workspace_id: Uuid,
) -> AuthResult<ImpersonatorParty> {
    let row: Option<(Uuid, Uuid, Uuid, bool, bool)> = sqlx::query_as(
        "SELECT uw.id, uw.\"userId\", uw.\"workspaceId\", u.\"canImpersonate\", \
                u.\"canAccessFullAdminPanel\" \
         FROM core.\"userWorkspace\" uw \
         JOIN core.\"user\" u ON u.id = uw.\"userId\" \
         WHERE uw.id = $1",
    )
    .bind(impersonator_user_workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (user_workspace_id, user_id, workspace_id, can_impersonate, is_full_admin) = row
        .ok_or_else(|| {
            user_workspace_not_found_error(
                "User not found in workspace or impersonation not enabled",
            )
        })?;

    Ok(ImpersonatorParty {
        user_workspace_id,
        user_id,
        workspace_id,
        can_impersonate,
        is_full_admin,
    })
}

async fn load_target_party(
    state: &AppState,
    target_user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<TargetParty> {
    let row: Option<(Uuid, Uuid, String, bool, bool, bool)> = sqlx::query_as(
        "SELECT uw.id, u.id, u.email::text, w.\"allowImpersonation\", \
                u.\"canImpersonate\", u.\"canAccessFullAdminPanel\" \
         FROM core.\"userWorkspace\" uw \
         JOIN core.\"user\" u ON u.id = uw.\"userId\" \
         JOIN core.\"workspace\" w ON w.id = uw.\"workspaceId\" \
         WHERE uw.\"userId\" = $1 AND uw.\"workspaceId\" = $2",
    )
    .bind(target_user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (user_workspace_id, user_id, email, allow_impersonation, can_impersonate, is_full_admin) =
        row.ok_or_else(|| {
            user_workspace_not_found_error(
                "User not found in workspace or impersonation not enabled",
            )
        })?;

    Ok(TargetParty {
        user_workspace_id,
        user_id,
        workspace_id,
        email,
        allow_impersonation,
        can_impersonate,
        is_full_admin,
    })
}

async fn load_target_party_by_email(
    state: &AppState,
    email: &str,
    workspace_id: Uuid,
) -> AuthResult<TargetParty> {
    let row: Option<(Uuid, Uuid, String, bool, bool, bool)> = sqlx::query_as(
        "SELECT uw.id, u.id, u.email::text, w.\"allowImpersonation\", \
                u.\"canImpersonate\", u.\"canAccessFullAdminPanel\" \
         FROM core.\"userWorkspace\" uw \
         JOIN core.\"user\" u ON u.id = uw.\"userId\" \
         JOIN core.\"workspace\" w ON w.id = uw.\"workspaceId\" \
         WHERE u.email = $1 AND uw.\"workspaceId\" = $2",
    )
    .bind(email)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (user_workspace_id, user_id, email, allow_impersonation, can_impersonate, is_full_admin) =
        row.ok_or_else(|| AuthError::forbidden("Impersonator or target user workspace not found"))?;

    Ok(TargetParty {
        user_workspace_id,
        user_id,
        email,
        workspace_id,
        allow_impersonation,
        can_impersonate,
        is_full_admin,
    })
}

fn require_creator_principal(context: &AuthContext) -> AuthResult<Uuid> {
    if context.api_key_id.is_some() {
        return Err(permission_denied(PERMISSION_DENIED_MESSAGE));
    }

    if !matches!(
        context.token_type,
        TokenType::Access | TokenType::WorkspaceAgnostic
    ) {
        return Err(permission_denied(PERMISSION_DENIED_MESSAGE));
    }

    if context.is_impersonating {
        return Err(permission_denied(PERMISSION_DENIED_MESSAGE));
    }

    context
        .user_id
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))
}

fn require_activator_principal(context: &AuthContext) -> AuthResult<(Uuid, Uuid)> {
    if context.api_key_id.is_some() {
        return Err(permission_denied(PERMISSION_DENIED_MESSAGE));
    }

    if !matches!(
        context.token_type,
        TokenType::Access | TokenType::Playground
    ) {
        return Err(permission_denied(PERMISSION_DENIED_MESSAGE));
    }

    let user_id = context
        .user_id
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    let workspace_id = context
        .workspace_id
        .ok_or_else(|| permission_denied(PERMISSION_DENIED_MESSAGE))?;

    Ok((user_id, workspace_id))
}

fn require_impersonator_principal(context: &AuthContext) -> AuthResult<(Uuid, Uuid, Uuid)> {
    if context.api_key_id.is_some() {
        return Err(permission_denied(IMPERSONATE_VIA_API_KEY_MESSAGE));
    }

    if context.token_type != TokenType::Access || context.is_impersonating {
        return Err(permission_denied(PERMISSION_DENIED_MESSAGE));
    }

    let user_id = context
        .user_id
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    let workspace_id = context
        .workspace_id
        .ok_or_else(|| permission_denied(PERMISSION_DENIED_MESSAGE))?;

    let user_workspace_id = context
        .user_workspace_id
        .ok_or_else(|| permission_denied(IMPERSONATE_VIA_API_KEY_MESSAGE))?;

    Ok((user_id, workspace_id, user_workspace_id))
}

async fn assert_subdomain_available(state: &AppState, subdomain: &str) -> AuthResult<()> {
    let taken: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM core.\"workspace\" WHERE subdomain = $1)")
            .bind(subdomain)
            .fetch_one(&state.core_pool)
            .await?;

    if taken.0 {
        return Err(subdomain_already_taken_error());
    }

    Ok(())
}

async fn generate_available_subdomain(
    state: &AppState,
    email: &str,
    display_name: &str,
) -> AuthResult<String> {
    let base = subdomain_from_email(email)
        .filter(|candidate| is_valid_subdomain(candidate))
        .or_else(|| subdomain_slug_from_display_name(display_name))
        .unwrap_or_else(generate_random_subdomain);

    let mut candidates = vec![base.clone()];

    for suffix in 2..MAX_NUMBERED_SUFFIX_ATTEMPTS {
        candidates.push(format!("{base}-{suffix}"));
    }

    for _ in 0..MAX_RANDOM_FALLBACK_ATTEMPTS {
        candidates.push(generate_random_subdomain());
    }

    for candidate in candidates {
        if !is_valid_subdomain(&candidate) {
            continue;
        }

        let taken: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM core.\"workspace\" WHERE subdomain = $1)")
                .bind(&candidate)
                .fetch_one(&state.core_pool)
                .await?;

        if !taken.0 {
            return Ok(candidate);
        }
    }

    Ok(generate_random_subdomain())
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

async fn create_workspace(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<CreateWorkspaceInput>,
) -> AuthResult<(StatusCode, Json<Value>)> {
    let user_id = require_creator_principal(&context)?;

    let display_name = input
        .display_name
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .ok_or_else(|| AuthError::invalid_input("Workspace name is required"))?;

    let user: (String, Option<String>) = sqlx::query_as(
        "SELECT email::text, locale FROM core.\"user\" \
         WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?
    .ok_or_else(|| AuthError::user_not_found("User not found"))?;

    let (email, locale) = user;

    let subdomain = match input.subdomain {
        Some(raw) => {
            let normalized = raw.trim().to_lowercase();

            if !is_valid_subdomain(&normalized) {
                return Err(subdomain_not_valid_error());
            }

            assert_subdomain_available(&state, &normalized).await?;

            normalized
        }
        None => generate_available_subdomain(&state, &email, &display_name).await?,
    };

    let workspace_id = Uuid::new_v4();
    let application_id = Uuid::new_v4();
    let invite_hash = Uuid::new_v4();
    let locale = locale.unwrap_or_else(|| "en".to_string());

    let mut transaction = state.core_pool.begin().await?;

    sqlx::query(
        "INSERT INTO core.\"workspace\" \
         (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
          \"workspaceCustomApplicationId\") \
         VALUES ($1, $2, $3, 'PENDING_CREATION', $4, $5)",
    )
    .bind(workspace_id)
    .bind(&display_name)
    .bind(invite_hash)
    .bind(&subdomain)
    .bind(application_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"application\" \
         (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", version, \
          \"canBeUninstalled\", \"workspaceId\") \
         VALUES ($1, $1, 'Custom', 'local', 'INSTALLED', 'workspace-custom', '1.0.1', false, $2)",
    )
    .bind(application_id)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"userWorkspace\" (\"userId\", \"workspaceId\", locale) \
         VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(workspace_id)
    .bind(&locale)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    let provider = context
        .auth_provider
        .as_deref()
        .and_then(AuthProvider::parse)
        .unwrap_or(AuthProvider::Password);

    let login_token = tokens::issue_login_token(&state, &email, workspace_id, provider).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "loginToken": login_token,
            "workspace": { "id": workspace_id },
        })),
    ))
}

async fn activate_workspace(
    State(state): State<AppState>,
    context: AuthContext,
) -> AuthResult<Json<Value>> {
    let (user_id, workspace_id) = require_activator_principal(&context)?;

    let workspace: (Uuid, Option<String>, String, String, Uuid) = sqlx::query_as(
        "SELECT id, \"displayName\", subdomain, \"activationStatus\"::text, \
                \"workspaceCustomApplicationId\" \
         FROM core.\"workspace\" WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?
    .ok_or_else(|| AuthError::workspace_not_found("Workspace not found"))?;

    let (id, display_name, subdomain, activation_status, application_id) = workspace;

    if activation_status == "CREATED" || activation_status == "ACTIVE" {
        if let Err(error) =
            crate::standard_objects::seed_standard_objects(&state, workspace_id).await
        {
            return Err(AuthError::internal(error.message));
        }

        return Ok(Json(json!({
            "workspace": {
                "id": id,
                "displayName": display_name,
                "subdomain": subdomain,
                "activationStatus": activation_status,
            },
        })));
    }

    let stale_threshold =
        Utc::now() - chrono::Duration::seconds(WORKSPACE_ACTIVATION_STALE_LOCK_SECS);

    let mut transaction = state.core_pool.begin().await?;

    let locked = sqlx::query(
        "UPDATE core.\"workspace\" SET \"activationStatus\" = 'ONGOING_CREATION', \
                \"updatedAt\" = now() \
         WHERE id = $1 AND (\"activationStatus\" = 'PENDING_CREATION' \
            OR (\"activationStatus\" = 'ONGOING_CREATION' AND \"updatedAt\" < $2))",
    )
    .bind(workspace_id)
    .bind(stale_threshold)
    .execute(&mut *transaction)
    .await?
    .rows_affected();

    if locked == 0 {
        transaction.rollback().await?;

        return Err(AuthError::internal("Workspace is already being created"));
    }

    let member_row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(&mut *transaction)
    .await?;

    let user_workspace_id = member_row.map(|(id,)| id).ok_or_else(|| {
        user_workspace_not_found_error("User not found in workspace or impersonation not enabled")
    })?;

    let schema_name = workspace_schema_name(&workspace_id);

    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name))
        .execute(&mut *transaction)
        .await?;

    let admin_role_id = Uuid::new_v4();
    let member_role_id = Uuid::new_v4();
    let custom_application_role_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO core.\"role\" \
         (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\", \
          \"canUpdateAllSettings\", \"canReadAllObjectRecords\", \"canCreateAllObjectRecords\", \
          \"canUpdateAllObjectRecords\", \"canSoftDeleteAllObjectRecords\", \
          \"canDestroyAllObjectRecords\") \
         VALUES ($1, $2, 'Admin', $3, $4, true, true, true, true, true, true)",
    )
    .bind(admin_role_id)
    .bind(uuid_from_universal_identifier(
        ADMIN_ROLE_UNIVERSAL_IDENTIFIER,
    )?)
    .bind(application_id)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"role\" \
         (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\", \
          \"canUpdateAllSettings\") \
         VALUES ($1, $2, $3, $4, $5, true)",
    )
    .bind(custom_application_role_id)
    .bind(uuid_from_universal_identifier(
        WORKSPACE_CUSTOM_APPLICATION_ROLE_UNIVERSAL_IDENTIFIER,
    )?)
    .bind(WORKSPACE_CUSTOM_APPLICATION_ROLE_LABEL)
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
    .bind(uuid_from_universal_identifier(
        MEMBER_ROLE_UNIVERSAL_IDENTIFIER,
    )?)
    .bind(application_id)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "UPDATE core.\"application\" SET \"requestedRoleId\" = $1, \"updatedAt\" = now() \
         WHERE id = $2",
    )
    .bind(custom_application_role_id)
    .bind(application_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "INSERT INTO core.\"roleTarget\" \
         (\"workspaceId\", \"roleId\", \"userWorkspaceId\", \"universalIdentifier\", \
          \"applicationId\") \
         SELECT $1, $2, $3, $4, w.\"workspaceCustomApplicationId\" \
         FROM core.\"workspace\" w WHERE w.id = $1",
    )
    .bind(workspace_id)
    .bind(admin_role_id)
    .bind(user_workspace_id)
    .bind(Uuid::new_v4())
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "UPDATE core.\"workspace\" \
         SET \"defaultRoleId\" = $1, \"databaseSchema\" = $2, \"activationStatus\" = 'CREATED', \
             \"updatedAt\" = now() \
         WHERE id = $3",
    )
    .bind(member_role_id)
    .bind(&schema_name)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    if let Err(error) = crate::standard_objects::seed_standard_objects(&state, workspace_id).await {
        tracing::error!(
            workspace_id = %workspace_id,
            error = %error.message,
            "workspace standard object seeding failed after activation"
        );

        return Err(AuthError::internal(error.message));
    }

    Ok(Json(json!({
        "workspace": {
            "id": workspace_id,
            "displayName": display_name,
            "subdomain": subdomain,
            "activationStatus": "CREATED",
        },
    })))
}

fn uuid_from_universal_identifier(raw: &str) -> AuthResult<Uuid> {
    Uuid::parse_str(raw).map_err(|error| AuthError::internal(error.to_string()))
}

async fn impersonate(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<ImpersonateInput>,
) -> AuthResult<Json<Value>> {
    let (user_id, workspace_id, user_workspace_id) = require_impersonator_principal(&context)?;

    let user_flags: (bool, bool) = sqlx::query_as(
        "SELECT \"canImpersonate\", \"canAccessFullAdminPanel\" \
         FROM core.\"user\" WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?
    .ok_or_else(|| AuthError::user_not_found("User not found"))?;

    let guard_passed = user_flags.0
        || user_workspace_has_permission_flag(&state, workspace_id, user_workspace_id).await?;

    if !guard_passed {
        return Err(permission_denied(IMPERSONATE_PERMISSION_MESSAGE));
    }

    let impersonator = load_impersonator_party(&state, user_workspace_id).await?;
    let target = load_target_party(&state, input.user_id, input.workspace_id).await?;

    if target.user_id == impersonator.user_id {
        return Err(AuthError::forbidden(SELF_IMPERSONATION_MESSAGE));
    }

    check_impersonation_authorization(&state, &impersonator, &target).await?;

    let claims = json!({
        "sub": target.email,
        "workspaceId": input.workspace_id,
        "authProvider": AuthProvider::Impersonation.as_str(),
        "impersonatorUserWorkspaceId": impersonator.user_workspace_id,
    });

    let login_token = state
        .jwt
        .sign(
            claims,
            TokenType::Login,
            state.config.login_token_expires_in_secs,
            Some(&Uuid::new_v4()),
        )
        .await?;

    Ok(Json(json!({
        "loginToken": login_token,
        "workspace": { "id": input.workspace_id },
    })))
}

async fn consume_login_token(state: &AppState, jwt_id: &str) -> AuthResult<()> {
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

    Ok(())
}

async fn issue_impersonation_token_pair(
    state: &AppState,
    target_user_id: Uuid,
    workspace_id: Uuid,
    target_user_workspace_id: Uuid,
    impersonator_user_workspace_id: Uuid,
) -> AuthResult<IssuedTokenPair> {
    let access_token = state
        .jwt
        .sign(
            impersonation_access_claims(
                target_user_id,
                workspace_id,
                target_user_workspace_id,
                impersonator_user_workspace_id,
                target_user_workspace_id,
            ),
            TokenType::Access,
            state.config.access_token_expires_in_secs,
            None,
        )
        .await?;

    let refresh_row: (Uuid,) = sqlx::query_as(
        "INSERT INTO core.\"appToken\" (\"userId\", \"workspaceId\", type, \"expiresAt\") \
         VALUES ($1, $2, 'REFRESH_TOKEN', now() + make_interval(secs => $3)) RETURNING id",
    )
    .bind(target_user_id)
    .bind(workspace_id)
    .bind(impersonation_refresh_lifetime_secs())
    .fetch_one(&state.core_pool)
    .await?;

    let refresh_token = state
        .jwt
        .sign(
            impersonation_refresh_claims(
                target_user_id,
                workspace_id,
                impersonator_user_workspace_id,
                target_user_workspace_id,
            ),
            TokenType::Refresh,
            impersonation_refresh_lifetime_secs() as u64,
            Some(&refresh_row.0),
        )
        .await?;

    Ok(IssuedTokenPair {
        access_token,
        refresh_token,
    })
}

async fn impersonation_tokens(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<LoginTokenInput>,
) -> AuthResult<Response> {
    let payload = state
        .jwt
        .verify_typed(&input.login_token, TokenType::Login)
        .await?;

    let provider = payload
        .get("authProvider")
        .and_then(Value::as_str)
        .and_then(AuthProvider::parse);

    if provider != Some(AuthProvider::Impersonation) {
        return Err(AuthError::unauthenticated("Token invalid."));
    }

    let token_workspace = payload
        .get("workspaceId")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok());

    if token_workspace != Some(input.workspace_id) {
        return Err(AuthError::workspace_not_found("Workspace not found"));
    }

    let email = payload
        .get("sub")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
        .to_string();

    let jwt_id = payload
        .get("jti")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
        .to_string();

    let impersonator_user_workspace_id = payload
        .get("impersonatorUserWorkspaceId")
        .and_then(Value::as_str)
        .filter(|raw| !raw.is_empty())
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .ok_or_else(|| {
            AuthError::forbidden("Impersonation login token does not name an impersonator")
        })?;

    let impersonator = load_impersonator_party(&state, impersonator_user_workspace_id)
        .await
        .map_err(|_| AuthError::forbidden("Impersonator or target user workspace not found"))?;

    let target = load_target_party_by_email(&state, &email, input.workspace_id)
        .await
        .map_err(|_| AuthError::forbidden("Impersonator or target user workspace not found"))?;

    if target.user_id == impersonator.user_id {
        return Err(AuthError::forbidden(SELF_IMPERSONATION_MESSAGE));
    }

    check_impersonation_authorization(&state, &impersonator, &target).await?;

    consume_login_token(&state, &jwt_id).await?;

    let issued = issue_impersonation_token_pair(
        &state,
        target.user_id,
        input.workspace_id,
        target.user_workspace_id,
        impersonator.user_workspace_id,
    )
    .await?;

    let context = session_context(&headers, Some(addr));
    let cookies =
        session::issue_session_for_token_pair(&state, &context, &issued, CreationOrigin::SignIn)
            .await?;

    Ok(response_with_cookies(
        StatusCode::OK,
        json!({
            "accessToken": issued.access_token,
            "refreshToken": issued.refresh_token,
            "user": { "id": target.user_id, "email": target.email },
        }),
        cookies,
    ))
}

async fn resolve_credential_payload(state: &AppState, headers: &HeaderMap) -> AuthResult<Value> {
    let bearer_token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty());

    if let Some(token) = bearer_token {
        if token.starts_with(session::USER_SESSION_TOKEN_PREFIX) {
            return Err(AuthError::unauthenticated(
                "Session tokens are only accepted from the session cookie",
            ));
        }

        return state.jwt.verify(token).await;
    }

    if let Some(session_token) = session::extract_session_token(headers, &state.config) {
        let resolved = session::resolve_session(state, &session_token).await?;

        return Ok(resolved.payload);
    }

    Err(AuthError::unauthenticated("Missing authorization header"))
}

async fn stop_impersonation(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AuthResult<Response> {
    let payload = resolve_credential_payload(&state, &headers).await?;

    let token_type = payload.get("type").and_then(Value::as_str);

    if token_type != Some(TokenType::Access.as_str())
        && token_type != Some(TokenType::Playground.as_str())
    {
        return Err(permission_denied(PERMISSION_DENIED_MESSAGE));
    }

    let is_impersonating = payload
        .get("isImpersonating")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let impersonator_user_workspace_id = if is_impersonating {
        payload
            .get("impersonatorUserWorkspaceId")
            .and_then(Value::as_str)
            .and_then(|raw| Uuid::parse_str(raw).ok())
    } else {
        None
    };

    let impersonator_user_workspace_id = impersonator_user_workspace_id
        .ok_or_else(|| AuthError::forbidden(NOT_IMPERSONATING_MESSAGE))?;

    let impersonator_row: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.\"userWorkspace\" WHERE id = $1")
            .bind(impersonator_user_workspace_id)
            .fetch_optional(&state.core_pool)
            .await?;

    if impersonator_row.is_none() {
        return Err(user_workspace_not_found_error(
            "Impersonator user workspace not found",
        ));
    }

    if let Some(presented) = session::extract_session_token(&headers, &state.config) {
        session::revoke_session_by_token(&state, &presented, RevocationReason::ImpersonationEnded)
            .await?;
    }

    let mut cookies = Vec::new();
    let mut can_restore = false;

    if let Some(parked_token) = session::extract_impersonator_session_token(&headers, &state.config)
    {
        if let Ok(resolved) = session::resolve_session(&state, &parked_token).await {
            if can_restore_impersonator_session(&resolved.payload, impersonator_user_workspace_id) {
                can_restore = true;

                cookies.push(session::session_cookie_value(
                    &state.config,
                    &parked_token,
                    resolved.expires_at,
                ));
            }
        }
    }

    if !can_restore {
        cookies.extend(session::session_cookie_clear_values(&state.config));
    }

    cookies.extend(session::impersonator_cookie_clear_values(&state.config));

    Ok(response_with_cookies(
        StatusCode::OK,
        json!({ "canRestoreImpersonatorSession": can_restore }),
        cookies,
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/workspace", post(create_workspace))
        .route("/auth/workspace/activate", post(activate_workspace))
        .route("/auth/impersonate", post(impersonate))
        .route("/auth/impersonation/tokens", post(impersonation_tokens))
        .route("/auth/stop-impersonation", post(stop_impersonation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn role_universal_identifiers_match_the_upstream_constants() {
        assert_eq!(
            ADMIN_ROLE_UNIVERSAL_IDENTIFIER,
            "20202020-02c2-43f2-b94d-cab1f2b532eb"
        );
        assert_eq!(
            MEMBER_ROLE_UNIVERSAL_IDENTIFIER,
            "20202020-a8a6-47ae-be79-e3601fa46a8e"
        );
        assert_eq!(
            WORKSPACE_CUSTOM_APPLICATION_ROLE_UNIVERSAL_IDENTIFIER,
            "b5b505e2-8834-457f-a301-f1f4694fe365"
        );
    }

    #[test]
    fn subdomain_rules_match_the_upstream_pattern_and_reserved_list() {
        assert!(is_valid_subdomain("nudge-ws-1"));
        assert!(is_valid_subdomain("a"));
        assert!(is_valid_subdomain(&"a".repeat(SUBDOMAIN_MAX_LENGTH)));
        assert!(is_valid_subdomain("2020"));

        assert!(!is_valid_subdomain("-starts-with-dash"));
        assert!(!is_valid_subdomain("ends-with-dash-"));
        assert!(!is_valid_subdomain("api-reserved-prefix"));
        assert!(!is_valid_subdomain(&"a".repeat(SUBDOMAIN_MAX_LENGTH + 1)));
        assert!(!is_valid_subdomain("UPPER-CASE"));
        assert!(!is_valid_subdomain("under_score"));
        assert!(!is_valid_subdomain(""));
        assert!(!is_valid_subdomain("admin"));
        assert!(!is_valid_subdomain("www"));
        assert!(!is_valid_subdomain("api"));
    }

    #[test]
    fn subdomain_slugs_derive_from_display_names_and_emails() {
        assert_eq!(
            subdomain_slug_from_display_name("Nudge HQ!"),
            Some("nudge-hq".to_string())
        );
        assert_eq!(subdomain_slug_from_display_name("  --  "), None);
        assert_eq!(
            subdomain_slug_from_display_name(&"x".repeat(SUBDOMAIN_MAX_LENGTH + 5)),
            Some("x".repeat(SUBDOMAIN_MAX_LENGTH))
        );
        assert_eq!(
            subdomain_from_email("team@acme.co"),
            Some("acme".to_string())
        );
        assert_eq!(subdomain_from_email("no-domain"), None);
    }

    #[test]
    fn random_subdomains_are_valid_and_numbered_candidates_stay_valid() {
        for _ in 0..20 {
            let generated = generate_random_subdomain();

            assert!(is_valid_subdomain(&generated), "{generated} must be valid");
        }

        assert!(is_valid_subdomain("base-2"));
        assert!(is_valid_subdomain("base-50"));
    }

    #[test]
    fn impersonation_levels_follow_the_workspace_boundary() {
        let left = Uuid::new_v4();
        let right = Uuid::new_v4();

        assert_eq!(
            impersonation_level(left, left),
            ImpersonationLevel::Workspace
        );
        assert_eq!(impersonation_level(left, right), ImpersonationLevel::Server);
    }

    #[test]
    fn target_guards_require_privilege_parity_with_the_impersonator() {
        assert!(user_can_server_impersonate_target(false, false));
        assert!(user_can_server_impersonate_target(true, false));
        assert!(user_can_server_impersonate_target(true, true));
        assert!(!user_can_server_impersonate_target(false, true));

        assert!(!user_has_admin_privileges(false, false));
        assert!(user_has_admin_privileges(true, false));
        assert!(user_has_admin_privileges(false, true));

        assert!(user_can_workspace_impersonate_target(
            (false, false),
            (false, false)
        ));
        assert!(!user_can_workspace_impersonate_target(
            (false, false),
            (true, false)
        ));
        assert!(!user_can_workspace_impersonate_target(
            (false, false),
            (false, true)
        ));
        assert!(user_can_workspace_impersonate_target(
            (true, false),
            (true, false)
        ));
        assert!(user_can_workspace_impersonate_target(
            (false, true),
            (true, true)
        ));
        assert!(!user_can_workspace_impersonate_target(
            (true, false),
            (false, true)
        ));
    }

    #[test]
    fn server_admin_workspace_gate_allows_unset_and_matches_case_insensitively() {
        let workspace_id = Uuid::new_v4();

        assert!(is_server_admin_access_allowed(workspace_id, None));
        assert!(is_server_admin_access_allowed(
            workspace_id,
            Some(&workspace_id.to_string())
        ));
        assert!(is_server_admin_access_allowed(
            workspace_id,
            Some(&workspace_id.to_string().to_uppercase())
        ));
        assert!(!is_server_admin_access_allowed(
            workspace_id,
            Some(&Uuid::new_v4().to_string())
        ));
    }

    #[test]
    fn impersonation_access_claims_mirror_the_upstream_shape() {
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let target_user_workspace_id = Uuid::new_v4();
        let impersonator_user_workspace_id = Uuid::new_v4();

        let claims = impersonation_access_claims(
            user_id,
            workspace_id,
            target_user_workspace_id,
            impersonator_user_workspace_id,
            target_user_workspace_id,
        );

        assert_eq!(claims["sub"], user_id.to_string());
        assert_eq!(claims["userId"], user_id.to_string());
        assert_eq!(claims["workspaceId"], workspace_id.to_string());
        assert_eq!(
            claims["userWorkspaceId"],
            target_user_workspace_id.to_string()
        );
        assert!(claims.get("type").is_none());
        assert_eq!(claims["authProvider"], "impersonation");
        assert_eq!(claims["isImpersonating"], true);
        assert_eq!(
            claims["impersonatorUserWorkspaceId"],
            impersonator_user_workspace_id.to_string()
        );
        assert_eq!(
            claims["impersonatedUserWorkspaceId"],
            target_user_workspace_id.to_string()
        );
        assert!(claims.get("workspaceMemberId").is_none());
    }

    #[test]
    fn impersonation_refresh_claims_target_access_and_live_one_day() {
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let target_user_workspace_id = Uuid::new_v4();
        let impersonator_user_workspace_id = Uuid::new_v4();

        let claims = impersonation_refresh_claims(
            user_id,
            workspace_id,
            impersonator_user_workspace_id,
            target_user_workspace_id,
        );

        assert_eq!(claims["sub"], user_id.to_string());
        assert_eq!(claims["userId"], user_id.to_string());
        assert_eq!(claims["workspaceId"], workspace_id.to_string());
        assert_eq!(claims["authProvider"], "impersonation");
        assert_eq!(claims["targetedTokenType"], "ACCESS");
        assert_eq!(claims["isImpersonating"], true);
        assert_eq!(
            claims["impersonatorUserWorkspaceId"],
            impersonator_user_workspace_id.to_string()
        );
        assert_eq!(
            claims["impersonatedUserWorkspaceId"],
            target_user_workspace_id.to_string()
        );
        assert!(claims.get("jti").is_none());
        assert!(claims.get("type").is_none());

        assert_eq!(impersonation_refresh_lifetime_secs(), 24 * 60 * 60);
    }

    #[test]
    fn restored_impersonator_sessions_require_a_matching_standard_payload() {
        let impersonator_user_workspace_id = Uuid::new_v4();
        let other_user_workspace_id = Uuid::new_v4();

        let standard = json!({
            "type": "ACCESS",
            "isImpersonating": false,
            "userWorkspaceId": impersonator_user_workspace_id.to_string(),
        });
        assert!(can_restore_impersonator_session(
            &standard,
            impersonator_user_workspace_id
        ));

        let still_impersonating = json!({
            "type": "ACCESS",
            "isImpersonating": true,
            "userWorkspaceId": impersonator_user_workspace_id.to_string(),
        });
        assert!(!can_restore_impersonator_session(
            &still_impersonating,
            impersonator_user_workspace_id
        ));

        let other_member = json!({
            "type": "ACCESS",
            "isImpersonating": false,
            "userWorkspaceId": other_user_workspace_id.to_string(),
        });
        assert!(!can_restore_impersonator_session(
            &other_member,
            impersonator_user_workspace_id
        ));

        let agnostic = json!({
            "type": "WORKSPACE_AGNOSTIC",
            "isImpersonating": false,
        });
        assert!(!can_restore_impersonator_session(
            &agnostic,
            impersonator_user_workspace_id
        ));

        let missing_claim = json!({
            "type": "ACCESS",
            "isImpersonating": false,
        });
        assert!(!can_restore_impersonator_session(
            &missing_claim,
            impersonator_user_workspace_id
        ));
    }

    #[test]
    fn denial_errors_keep_the_upstream_codes_and_statuses() {
        let server_level = server_level_not_allowed_error();
        assert_eq!(server_level.status, 403);
        assert_eq!(server_level.code, "FORBIDDEN_EXCEPTION");

        let workspace_level = workspace_level_not_allowed_error();
        assert_eq!(workspace_level.status, 403);

        let target_admin = target_admin_privileges_error();
        assert_eq!(target_admin.status, 403);

        let missing_row = user_workspace_not_found_error("User not found in workspace");
        assert_eq!(missing_row.status, 500);
        assert_eq!(missing_row.code, "USER_WORKSPACE_NOT_FOUND");

        let not_valid = subdomain_not_valid_error();
        assert_eq!(not_valid.status, 400);
        assert_eq!(not_valid.code, "SUBDOMAIN_NOT_VALID");

        let taken = subdomain_already_taken_error();
        assert_eq!(taken.status, 400);
        assert_eq!(taken.code, "SUBDOMAIN_ALREADY_TAKEN");

        let permission = permission_denied(PERMISSION_DENIED_MESSAGE);
        assert_eq!(permission.status, 403);
        assert_eq!(permission.code, "PERMISSION_DENIED");
    }

    #[test]
    fn impersonation_login_token_claims_name_the_target_and_impersonator() {
        let workspace_id = Uuid::new_v4();
        let impersonator_user_workspace_id = Uuid::new_v4();

        let claims = json!({
            "sub": "target@nudge.test",
            "workspaceId": workspace_id,
            "authProvider": AuthProvider::Impersonation.as_str(),
            "impersonatorUserWorkspaceId": impersonator_user_workspace_id,
        });

        let claims_text = claims.to_string();
        let parsed: Value = serde_json::from_str(&claims_text).unwrap();

        assert_eq!(parsed["sub"], "target@nudge.test");
        assert_eq!(parsed["workspaceId"], workspace_id.to_string());
        assert_eq!(parsed["authProvider"], "impersonation");
        assert_eq!(
            parsed["impersonatorUserWorkspaceId"],
            impersonator_user_workspace_id.to_string()
        );
    }
}
