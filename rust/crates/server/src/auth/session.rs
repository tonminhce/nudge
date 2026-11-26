use axum::http::{HeaderMap, HeaderValue};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, TimeZone, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::error::{AuthError, AuthResult};
use super::tokens::IssuedTokenPair;
use crate::state::AppState;

pub const USER_SESSION_TOKEN_PREFIX: &str = "sess_";
pub const SECURE_COOKIE_NAME: &str = "__Host-twenty-session";
pub const INSECURE_COOKIE_NAME: &str = "twenty-session";
pub const SECURE_IMPERSONATOR_COOKIE_NAME: &str = "__Host-twenty-impersonator-session";
pub const INSECURE_IMPERSONATOR_COOKIE_NAME: &str = "twenty-impersonator-session";
pub const SESSION_COOKIE_OPT_IN_HEADER: &str = "x-twenty-session-cookie";
pub const CSRF_ORIGIN_MISMATCH_MESSAGE: &str =
    "Request origin is not allowed for cookie-authenticated requests";
pub const SESSION_COOKIE_ISSUE_FORBIDDEN_MESSAGE: &str =
    "Request origin is not allowed to receive a session cookie";

const CACHE_KEY_NAMESPACE: &str = "engine:auth-session";
const CACHE_TTL_SECS: u64 = 60;
const MAX_TOUCH_INTERVAL_SECS: i64 = 5 * 60;
const IMPERSONATION_LIFETIME_SECS: i64 = 24 * 60 * 60;
const CLEANUP_RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
const CLEANUP_BATCH_SIZE: i64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationOrigin {
    SignIn,
    RenewalBridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationReason {
    UserSignOut,
    Superseded,
    PasswordChanged,
}

impl RevocationReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RevocationReason::UserSignOut => "USER_SIGN_OUT",
            RevocationReason::Superseded => "SUPERSEDED",
            RevocationReason::PasswordChanged => "PASSWORD_CHANGED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieAccess {
    OptedIn,
    Navigation,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub user_workspace_id: Option<Uuid>,
    pub auth_provider: String,
    pub is_impersonating: bool,
    pub impersonator_user_workspace_id: Option<Uuid>,
    pub impersonated_user_workspace_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub authenticated_at: DateTime<Utc>,
}

pub struct ResolvedSession {
    pub payload: Value,
    #[expect(dead_code, reason = "surfaced by the sessions UI in a later wave")]
    pub authenticated_at: DateTime<Utc>,
    #[expect(dead_code, reason = "surfaced by the sessions UI in a later wave")]
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
struct CreateSessionInput {
    user_id: Uuid,
    workspace_id: Option<Uuid>,
    user_workspace_id: Option<Uuid>,
    auth_provider: String,
    is_impersonating: bool,
    impersonator_user_workspace_id: Option<Uuid>,
    impersonated_user_workspace_id: Option<Uuid>,
    user_agent: Option<String>,
    ip_address: Option<String>,
    country: Option<String>,
    region: Option<String>,
    city: Option<String>,
}

pub struct SessionRequestContext<'a> {
    pub headers: &'a HeaderMap,
    pub ip_address: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    #[sqlx(rename = "userId")]
    user_id: Uuid,
    #[sqlx(rename = "workspaceId")]
    workspace_id: Option<Uuid>,
    #[sqlx(rename = "userWorkspaceId")]
    user_workspace_id: Option<Uuid>,
    #[sqlx(rename = "authProvider")]
    auth_provider: String,
    #[sqlx(rename = "isImpersonating")]
    is_impersonating: bool,
    #[sqlx(rename = "impersonatorUserWorkspaceId")]
    impersonator_user_workspace_id: Option<Uuid>,
    #[sqlx(rename = "impersonatedUserWorkspaceId")]
    impersonated_user_workspace_id: Option<Uuid>,
    #[sqlx(rename = "expiresAt")]
    expires_at: DateTime<Utc>,
    #[sqlx(rename = "lastActiveAt")]
    last_active_at: DateTime<Utc>,
    #[sqlx(rename = "createdAt")]
    created_at: DateTime<Utc>,
    #[sqlx(rename = "revokedAt")]
    revoked_at: Option<DateTime<Utc>>,
    #[sqlx(rename = "revokedReason")]
    revoked_reason: Option<String>,
}

const SESSION_ROW_COLUMNS: &str = "id, \"userId\", \"workspaceId\", \"userWorkspaceId\", \
     \"authProvider\", \"isImpersonating\", \"impersonatorUserWorkspaceId\", \
     \"impersonatedUserWorkspaceId\", \"expiresAt\", \"lastActiveAt\", \"createdAt\", \
     \"revokedAt\", \"revokedReason\"";

impl SessionRow {
    fn to_cached(&self) -> CachedSession {
        CachedSession {
            session_id: self.id,
            user_id: self.user_id,
            workspace_id: self.workspace_id,
            user_workspace_id: self.user_workspace_id,
            auth_provider: self.auth_provider.clone(),
            is_impersonating: self.is_impersonating,
            impersonator_user_workspace_id: self.impersonator_user_workspace_id,
            impersonated_user_workspace_id: self.impersonated_user_workspace_id,
            expires_at: self.expires_at,
            last_active_at: self.last_active_at,
            authenticated_at: self.created_at,
        }
    }

    fn is_active(&self, config: &nudge_core::Config) -> bool {
        self.revoked_at.is_none() && is_cached_session_active(&self.to_cached(), config)
    }
}

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);

    format!(
        "{USER_SESSION_TOKEN_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(bytes)
    )
}

pub fn hash_session_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn invalid_session_error() -> AuthError {
    AuthError::unauthenticated("Session is invalid or has expired.")
}

fn superseded_session_error() -> AuthError {
    AuthError::new(
        401,
        "SESSION_SUPERSEDED",
        "Session was replaced by a newer sign-in.",
    )
}

fn revoked_session_error(reason: Option<&str>) -> AuthError {
    if reason == Some(RevocationReason::Superseded.as_str()) {
        superseded_session_error()
    } else {
        invalid_session_error()
    }
}

pub fn resolve_cookie_access(headers: &HeaderMap) -> CookieAccess {
    if headers.contains_key(SESSION_COOKIE_OPT_IN_HEADER) {
        return CookieAccess::OptedIn;
    }

    if headers
        .get("sec-fetch-mode")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|mode| mode == "navigate")
    {
        return CookieAccess::Navigation;
    }

    CookieAccess::Denied
}

pub fn is_secure_deployment(config: &nudge_core::Config) -> bool {
    let https_server_url = config
        .server_url
        .to_ascii_lowercase()
        .starts_with("https://");

    https_server_url || config.auth_cookie_same_site == "none"
}

fn read_cookie_value(cookie_header: &str, cookie_name: &str) -> Option<String> {
    for cookie_part in cookie_header.split(';') {
        let Some(separator_index) = cookie_part.find('=') else {
            continue;
        };

        if cookie_part[..separator_index].trim() != cookie_name {
            continue;
        }

        let value = cookie_part[separator_index + 1..].trim();

        if !value.is_empty() {
            return Some(value.to_string());
        }
    }

    None
}

pub fn extract_session_token(headers: &HeaderMap, config: &nudge_core::Config) -> Option<String> {
    if resolve_cookie_access(headers) == CookieAccess::Denied {
        return None;
    }

    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())?;

    let allow_insecure_name = !is_secure_deployment(config);

    let token = read_cookie_value(cookie_header, SECURE_COOKIE_NAME).or_else(|| {
        if allow_insecure_name {
            read_cookie_value(cookie_header, INSECURE_COOKIE_NAME)
        } else {
            None
        }
    })?;

    if token.starts_with(USER_SESSION_TOKEN_PREFIX) {
        Some(token)
    } else {
        None
    }
}

pub fn has_session_cookie(headers: &HeaderMap) -> bool {
    if resolve_cookie_access(headers) == CookieAccess::Denied {
        return false;
    }

    let Some(cookie_header) = headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };

    [SECURE_COOKIE_NAME, INSECURE_COOKIE_NAME]
        .iter()
        .any(|name| {
            cookie_header
                .split(';')
                .any(|part| part.trim().starts_with(&format!("{name}=")))
        })
}

fn same_site_attribute(config: &nudge_core::Config) -> String {
    match config.auth_cookie_same_site.as_str() {
        "strict" => "SameSite=Strict".to_string(),
        "none" => "SameSite=None".to_string(),
        _ => "SameSite=Lax".to_string(),
    }
}

fn secure_attribute(config: &nudge_core::Config) -> &'static str {
    if is_secure_deployment(config) {
        "; Secure"
    } else {
        ""
    }
}

pub fn session_cookie_value(
    config: &nudge_core::Config,
    token: &str,
    expires_at: DateTime<Utc>,
) -> HeaderValue {
    let name = if is_secure_deployment(config) {
        SECURE_COOKIE_NAME
    } else {
        INSECURE_COOKIE_NAME
    };
    let expires = expires_at.format("%a, %d %b %Y %H:%M:%S GMT");

    HeaderValue::from_str(&format!(
        "{name}={token}; Path=/; Expires={expires}; HttpOnly; {};{}",
        same_site_attribute(config),
        secure_attribute(config)
    ))
    .expect("cookie header value is ascii")
}

pub fn session_cookie_clear_values(config: &nudge_core::Config) -> Vec<HeaderValue> {
    let suffix = format!(
        "=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; {};{}",
        same_site_attribute(config),
        secure_attribute(config)
    );

    [SECURE_COOKIE_NAME, INSECURE_COOKIE_NAME]
        .iter()
        .map(|name| {
            HeaderValue::from_str(&format!("{name}{suffix}")).expect("cookie header is ascii")
        })
        .collect()
}

pub fn impersonator_cookie_value(config: &nudge_core::Config, token: &str) -> HeaderValue {
    let name = if is_secure_deployment(config) {
        SECURE_IMPERSONATOR_COOKIE_NAME
    } else {
        INSECURE_IMPERSONATOR_COOKIE_NAME
    };

    HeaderValue::from_str(&format!(
        "{name}={token}; Path=/; HttpOnly; {};{}",
        same_site_attribute(config),
        secure_attribute(config)
    ))
    .expect("cookie header value is ascii")
}

fn parse_url_parts(raw: &str) -> Option<(String, String, Option<String>)> {
    let (scheme, rest) = raw.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();

    if scheme != "http" && scheme != "https" {
        return None;
    }

    let authority = rest.split(['/', '?', '#']).next()?;

    let authority = match authority.rsplit_once('@') {
        Some((userinfo, host)) if !userinfo.is_empty() && !host.is_empty() => host,
        _ => authority,
    };

    if authority.is_empty() {
        return None;
    }

    let default_port = if scheme == "https" { "443" } else { "80" };

    let normalize_port = |port: Option<&str>| -> Option<Option<String>> {
        match port {
            Some(port) if !port.is_empty() => {
                if port.parse::<u16>().is_err() {
                    return Some(None);
                }

                if port == default_port {
                    Some(None)
                } else {
                    Some(Some(port.to_string()))
                }
            }
            _ => Some(None),
        }
    };

    if let Some(inner) = authority.strip_prefix('[') {
        let close = inner.find(']')?;
        let host = format!("[{}]", &inner[..close]).to_ascii_lowercase();
        let after = &inner[close + 1..];

        return normalize_port(after.strip_prefix(':')).map(|port| (scheme, host, port));
    }

    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };

    if host.is_empty() {
        return None;
    }

    normalize_port(port).map(|port| (scheme, host.to_ascii_lowercase(), port))
}

fn origin_string(scheme: &str, host: &str, port: Option<&str>) -> String {
    match port {
        Some(port) => format!("{scheme}://{host}:{port}"),
        None => format!("{scheme}://{host}"),
    }
}

pub fn to_comparable_origin(raw: &str) -> Option<String> {
    let (scheme, host, port) = parse_url_parts(raw)?;

    Some(origin_string(&scheme, &host, port.as_deref()))
}

fn is_ipv4_loopback(host: &str) -> bool {
    let octets: Vec<&str> = host.split('.').collect();

    octets.len() == 4
        && octets[0] == "127"
        && octets[1..].iter().all(|octet| {
            (1..=3).contains(&octet.len()) && octet.chars().all(|c| c.is_ascii_digit())
        })
}

fn is_mapped_hex_loopback(host: &str) -> bool {
    let Some(rest) = host.strip_prefix("::ffff:") else {
        return false;
    };

    let Some((first, second)) = rest.split_once(':') else {
        return false;
    };

    let parse_hextet = |raw: &str| -> Option<u32> {
        if !(1..=4).contains(&raw.len()) || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        u32::from_str_radix(raw, 16).ok()
    };

    let first = match parse_hextet(first) {
        Some(value) => value,
        None => return false,
    };

    if parse_hextet(second).is_none() {
        return false;
    }

    (first >> 8) == 127
}

pub fn is_loopback_hostname(hostname: &str) -> bool {
    let host = hostname
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.')
        .to_ascii_lowercase();

    host == "localhost" || host == "::1" || is_ipv4_loopback(&host) || is_mapped_hex_loopback(&host)
}

fn is_loopback_origin(origin: &str) -> bool {
    parse_url_parts(origin)
        .map(|(_, host, _)| is_loopback_hostname(&host))
        .unwrap_or(false)
}

fn request_base_origin(headers: &HeaderMap) -> Option<String> {
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|value| value.to_str().ok())
        .filter(|host| !host.is_empty())?;

    let forwarded_proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|proto| !proto.is_empty());

    let scheme = forwarded_proto.unwrap_or("http").to_ascii_lowercase();

    let (parsed_scheme, parsed_host, parsed_port) = parse_url_parts(&format!("{scheme}://{host}"))?;

    Some(origin_string(
        &parsed_scheme,
        &parsed_host,
        parsed_port.as_deref(),
    ))
}

pub fn allowed_credentialed_origins(config: &nudge_core::Config) -> Vec<String> {
    let explicit: Vec<String> = config
        .auth_cookie_allowed_origins
        .split(',')
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(String::from)
        .collect();

    let mut allowed = Vec::new();

    let derived = [config.server_url.as_str(), config.frontend_url.as_str()];

    let candidates: Vec<&str> = derived
        .into_iter()
        .chain(explicit.iter().map(String::as_str))
        .collect();

    for candidate in candidates {
        if candidate.is_empty() {
            continue;
        }

        let Some(origin) = to_comparable_origin(candidate) else {
            continue;
        };

        if config.node_env.is_production()
            && is_loopback_origin(&origin)
            && !explicit.iter().any(|raw| raw.as_str() == candidate)
        {
            continue;
        }

        if !allowed.contains(&origin) {
            allowed.push(origin);
        }
    }

    allowed
}

pub fn is_request_origin_allowed(
    headers: &HeaderMap,
    config: &nudge_core::Config,
    origin: &str,
) -> bool {
    let normalized_origin = origin.to_lowercase();

    if let Some(comparable_origin) = to_comparable_origin(&normalized_origin) {
        if let Some(request_origin) = request_base_origin(headers) {
            if comparable_origin == request_origin {
                return true;
            }
        }
    }

    allowed_credentialed_origins(config).contains(&normalized_origin)
}

fn extract_origin(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .filter(|origin| !origin.is_empty())
        .map(String::from)
}

pub fn is_request_allowed_to_receive_session_cookie(
    headers: &HeaderMap,
    config: &nudge_core::Config,
) -> bool {
    match extract_origin(headers) {
        None => true,
        Some(origin) => is_request_origin_allowed(headers, config, &origin),
    }
}

fn cloudflare_geolocation(headers: &HeaderMap) -> (Option<String>, Option<String>, Option<String>) {
    let read_utf8 = |name: &str| -> Option<String> {
        let raw = headers.get(name)?.as_bytes().to_vec();

        let decoded = String::from_utf8(raw).ok()?;
        let trimmed = decoded.trim();

        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };

    let country = read_utf8("cf-ipcountry")
        .map(|value| value.to_uppercase())
        .filter(|value| {
            value.len() == 2
                && value.chars().all(|c| c.is_ascii_uppercase())
                && value != "XX"
                && value != "T1"
        });

    (country, read_utf8("cf-region"), read_utf8("cf-ipcity"))
}

async fn redis_connection(state: &AppState) -> AuthResult<redis::aio::MultiplexedConnection> {
    state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

async fn cache_get(
    connection: &mut redis::aio::MultiplexedConnection,
    token_hash: &str,
) -> AuthResult<Option<CachedSession>> {
    let raw: Option<String> = redis::cmd("GET")
        .arg(format!("{CACHE_KEY_NAMESPACE}:{token_hash}"))
        .query_async(connection)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    let raw = match raw {
        Some(raw) => raw,
        None => return Ok(None),
    };

    match serde_json::from_str(&raw) {
        Ok(cached) => Ok(Some(cached)),
        Err(_) => {
            cache_del(&mut *connection, token_hash).await?;

            Ok(None)
        }
    }
}

async fn cache_set(
    connection: &mut redis::aio::MultiplexedConnection,
    token_hash: &str,
    cached: &CachedSession,
) -> AuthResult<()> {
    let serialized =
        serde_json::to_string(cached).map_err(|error| AuthError::internal(error.to_string()))?;

    redis::cmd("SETEX")
        .arg(format!("{CACHE_KEY_NAMESPACE}:{token_hash}"))
        .arg(CACHE_TTL_SECS)
        .arg(serialized)
        .query_async::<()>(&mut *connection)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

async fn cache_del(
    connection: &mut redis::aio::MultiplexedConnection,
    token_hash: &str,
) -> AuthResult<()> {
    redis::cmd("DEL")
        .arg(format!("{CACHE_KEY_NAMESPACE}:{token_hash}"))
        .query_async::<()>(&mut *connection)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

fn touch_interval_secs(config: &nudge_core::Config) -> i64 {
    (config.session_idle_timeout_secs as i64 / 2).min(MAX_TOUCH_INTERVAL_SECS)
}

fn is_cached_session_active(cached: &CachedSession, config: &nudge_core::Config) -> bool {
    let now = Utc::now();

    cached.expires_at > now
        && cached.last_active_at + Duration::seconds(config.session_idle_timeout_secs as i64) > now
}

async fn assert_not_revoked_after_caching(
    state: &AppState,
    session_id: Uuid,
    token_hash: &str,
) -> AuthResult<()> {
    let revoked: Option<(Option<DateTime<Utc>>, Option<String>)> = sqlx::query_as(
        "SELECT \"revokedAt\", \"revokedReason\" FROM core.\"userSession\" WHERE id = $1",
    )
    .bind(session_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (revoked_at, revoked_reason) = revoked.ok_or_else(invalid_session_error)?;

    if revoked_at.is_some() {
        let mut connection = redis_connection(state).await?;
        cache_del(&mut connection, token_hash).await?;

        return Err(revoked_session_error(revoked_reason.as_deref()));
    }

    Ok(())
}

async fn touch_session_if_due(
    state: &AppState,
    token_hash: &str,
    mut cached: CachedSession,
) -> AuthResult<bool> {
    let now = Utc::now();

    if (now - cached.last_active_at).num_seconds() < touch_interval_secs(&state.config) {
        return Ok(false);
    }

    let affected = sqlx::query(
        "UPDATE core.\"userSession\" SET \"lastActiveAt\" = $1 \
         WHERE id = $2 AND \"revokedAt\" IS NULL",
    )
    .bind(now)
    .bind(cached.session_id)
    .execute(&state.core_pool)
    .await?
    .rows_affected();

    if affected == 0 {
        let mut connection = redis_connection(state).await?;
        cache_del(&mut connection, token_hash).await?;

        let reason: Option<(Option<String>,)> =
            sqlx::query_as("SELECT \"revokedReason\" FROM core.\"userSession\" WHERE id = $1")
                .bind(cached.session_id)
                .fetch_optional(&state.core_pool)
                .await?;

        return Err(revoked_session_error(
            reason.and_then(|(reason,)| reason).as_deref(),
        ));
    }

    cached.last_active_at = now;

    let mut connection = redis_connection(state).await?;
    cache_set(&mut connection, token_hash, &cached).await?;
    assert_not_revoked_after_caching(state, cached.session_id, token_hash).await?;

    Ok(true)
}

pub fn build_session_payload(cached: &CachedSession) -> AuthResult<Value> {
    let Some(workspace_id) = cached.workspace_id else {
        return Ok(serde_json::json!({
            "sub": cached.user_id,
            "userId": cached.user_id,
            "authProvider": cached.auth_provider,
            "type": "WORKSPACE_AGNOSTIC",
        }));
    };

    let Some(user_workspace_id) = cached.user_workspace_id else {
        return Err(invalid_session_error());
    };

    let mut payload = serde_json::json!({
        "sub": cached.user_id,
        "userId": cached.user_id,
        "workspaceId": workspace_id,
        "userWorkspaceId": user_workspace_id,
        "authProvider": cached.auth_provider,
        "type": "ACCESS",
        "isImpersonating": cached.is_impersonating,
    });

    if cached.is_impersonating {
        payload["impersonatorUserWorkspaceId"] = cached
            .impersonator_user_workspace_id
            .map(|id| Value::from(id.to_string()))
            .unwrap_or(Value::Null);
        payload["impersonatedUserWorkspaceId"] = cached
            .impersonated_user_workspace_id
            .map(|id| Value::from(id.to_string()))
            .unwrap_or(Value::Null);
    }

    Ok(payload)
}

async fn fetch_session_row(state: &AppState, token_hash: &str) -> AuthResult<Option<SessionRow>> {
    let row: Option<SessionRow> = sqlx::query_as(&format!(
        "SELECT {SESSION_ROW_COLUMNS} FROM core.\"userSession\" WHERE \"tokenHash\" = $1"
    ))
    .bind(token_hash)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

pub async fn resolve_session(state: &AppState, session_token: &str) -> AuthResult<ResolvedSession> {
    let token_hash = hash_session_token(session_token);

    let mut connection = redis_connection(state).await?;

    if let Some(cached) = cache_get(&mut connection, &token_hash).await? {
        if !is_cached_session_active(&cached, &state.config) {
            cache_del(&mut connection, &token_hash).await?;

            return Err(invalid_session_error());
        }

        touch_session_if_due(state, &token_hash, cached.clone()).await?;

        return Ok(ResolvedSession {
            payload: build_session_payload(&cached)?,
            authenticated_at: cached.authenticated_at,
            expires_at: cached.expires_at,
        });
    }

    let row = fetch_session_row(state, &token_hash)
        .await?
        .ok_or_else(invalid_session_error)?;

    if !row.is_active(&state.config) {
        return Err(revoked_session_error(row.revoked_reason.as_deref()));
    }

    let cached = row.to_cached();

    let was_cached_by_touch = touch_session_if_due(state, &token_hash, cached.clone()).await?;

    if !was_cached_by_touch {
        cache_set(&mut connection, &token_hash, &cached).await?;
        assert_not_revoked_after_caching(state, cached.session_id, &token_hash).await?;
    }

    Ok(ResolvedSession {
        payload: build_session_payload(&cached)?,
        authenticated_at: cached.authenticated_at,
        expires_at: cached.expires_at,
    })
}

pub async fn revoke_session_by_token(
    state: &AppState,
    session_token: &str,
    reason: RevocationReason,
) -> AuthResult<bool> {
    let token_hash = hash_session_token(session_token);

    let Some(row) = fetch_session_row(state, &token_hash).await? else {
        return Ok(false);
    };

    let affected = sqlx::query(
        "UPDATE core.\"userSession\" \
         SET \"revokedAt\" = now(), \"revokedReason\" = $1, \"updatedAt\" = now() \
         WHERE id = $2 AND \"revokedAt\" IS NULL",
    )
    .bind(reason.as_str())
    .bind(row.id)
    .execute(&state.core_pool)
    .await?
    .rows_affected();

    let mut connection = redis_connection(state).await?;
    cache_del(&mut connection, &token_hash).await?;

    Ok(affected == 1)
}

pub async fn revoke_all_sessions_for_user(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Option<Uuid>,
    reason: RevocationReason,
    except_session_id: Option<Uuid>,
) -> AuthResult<u64> {
    let mut query = String::from(
        "UPDATE core.\"userSession\" \
         SET \"revokedAt\" = now(), \"revokedReason\" = $1, \"updatedAt\" = now() \
         WHERE \"userId\" = $2 AND \"revokedAt\" IS NULL",
    );

    let mut has_workspace = false;
    let mut has_except = false;

    if workspace_id.is_some() {
        query.push_str(" AND \"workspaceId\" = $3");
        has_workspace = true;
    }

    if except_session_id.is_some() {
        match has_workspace {
            true => {
                query.push_str(" AND id != $4");
                has_except = true;
            }
            false => {
                query.push_str(" AND id != $3");
                has_except = true;
            }
        }
    }

    query.push_str(" RETURNING \"tokenHash\"");

    let mut statement = sqlx::query_as::<_, (String,)>(&query)
        .bind(reason.as_str())
        .bind(user_id);

    if has_workspace {
        statement = statement.bind(workspace_id);
    }

    if has_except {
        statement = statement.bind(except_session_id);
    }

    let revoked: Vec<(String,)> = statement.fetch_all(&state.core_pool).await?;

    let count = revoked.len() as u64;

    if count > 0 {
        let mut connection = redis_connection(state).await?;

        for (token_hash,) in revoked {
            cache_del(&mut connection, &token_hash).await?;
        }
    }

    Ok(count)
}

pub async fn sign_out(
    state: &AppState,
    session_token: Option<&str>,
    refresh_token: Option<&str>,
) -> AuthResult<()> {
    if let Some(session_token) = session_token {
        revoke_session_by_token(state, session_token, RevocationReason::UserSignOut).await?;
    }

    if let Some(refresh_token) = refresh_token {
        revoke_presented_refresh_token(state, refresh_token).await?;
    }

    Ok(())
}

async fn revoke_presented_refresh_token(state: &AppState, refresh_token: &str) -> AuthResult<()> {
    let payload = match state
        .jwt
        .verify_typed(refresh_token, super::jwt_service::TokenType::Refresh)
        .await
    {
        Ok(payload) => payload,
        Err(_) => return Ok(()),
    };

    let Some(jwt_id) = payload
        .get("jti")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
    else {
        return Ok(());
    };

    sqlx::query(
        "UPDATE core.\"appToken\" SET \"revokedAt\" = now(), \"updatedAt\" = now() \
         WHERE id = $1 AND type = 'REFRESH_TOKEN' AND \"revokedAt\" IS NULL",
    )
    .bind(jwt_id)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

fn build_create_session_input(
    token_pair: &IssuedTokenPair,
    context: &SessionRequestContext<'_>,
    geolocation_enabled: bool,
) -> Option<CreateSessionInput> {
    let payload = super::jwt_service::JwtService::decode_payload(&token_pair.access_token)?;

    let payload_type = payload.get("type").and_then(Value::as_str)?;

    let user_agent = context
        .headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(String::from);

    let (country, region, city) = if geolocation_enabled {
        cloudflare_geolocation(context.headers)
    } else {
        (None, None, None)
    };

    let auth_provider = payload
        .get("authProvider")
        .and_then(Value::as_str)
        .map(String::from)?;

    let user_id = payload
        .get("userId")
        .or_else(|| payload.get("sub"))
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())?;

    if payload_type == "WORKSPACE_AGNOSTIC" {
        return Some(CreateSessionInput {
            user_id,
            workspace_id: None,
            user_workspace_id: None,
            auth_provider,
            is_impersonating: false,
            impersonator_user_workspace_id: None,
            impersonated_user_workspace_id: None,
            user_agent,
            ip_address: context.ip_address.clone(),
            country,
            region,
            city,
        });
    }

    if payload_type != "ACCESS" {
        return None;
    }

    let workspace_id = payload
        .get("workspaceId")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok());

    let user_workspace_id = payload
        .get("userWorkspaceId")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok());

    let is_impersonating = payload
        .get("isImpersonating")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Some(CreateSessionInput {
        user_id,
        workspace_id,
        user_workspace_id,
        auth_provider,
        is_impersonating,
        impersonator_user_workspace_id: payload
            .get("impersonatorUserWorkspaceId")
            .and_then(Value::as_str)
            .and_then(|raw| Uuid::parse_str(raw).ok()),
        impersonated_user_workspace_id: payload
            .get("impersonatedUserWorkspaceId")
            .and_then(Value::as_str)
            .and_then(|raw| Uuid::parse_str(raw).ok()),
        user_agent,
        ip_address: context.ip_address.clone(),
        country,
        region,
        city,
    })
}

async fn create_session(
    state: &AppState,
    input: &CreateSessionInput,
) -> AuthResult<(String, DateTime<Utc>)> {
    if input.workspace_id.is_some() && input.user_workspace_id.is_none() {
        return Err(AuthError::invalid_input(
            "Cannot create a workspace session without a user workspace",
        ));
    }

    if input.workspace_id.is_none() && input.user_workspace_id.is_some() {
        return Err(AuthError::invalid_input(
            "Cannot create a workspace-agnostic session for a user workspace",
        ));
    }

    if input.is_impersonating
        && (input.workspace_id.is_none()
            || input.impersonator_user_workspace_id.is_none()
            || input.impersonated_user_workspace_id.is_none())
    {
        return Err(AuthError::invalid_input(
            "Cannot create an impersonation session without a workspace and both user workspaces",
        ));
    }

    let lifetime_secs = if input.is_impersonating {
        IMPERSONATION_LIFETIME_SECS
    } else {
        state.config.session_absolute_lifetime_secs as i64
    };

    let now = Utc::now();
    let session_token = generate_session_token();
    let expires_at = now + Duration::seconds(lifetime_secs);

    sqlx::query(
        "INSERT INTO core.\"userSession\" \
         (\"tokenHash\", \"userId\", \"workspaceId\", \"userWorkspaceId\", \"authProvider\", \
          \"isImpersonating\", \"impersonatorUserWorkspaceId\", \"impersonatedUserWorkspaceId\", \
          \"userAgent\", \"ipAddress\", \"country\", \"region\", \"city\", \
          \"expiresAt\", \"lastActiveAt\") \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(hash_session_token(&session_token))
    .bind(input.user_id)
    .bind(input.workspace_id)
    .bind(input.user_workspace_id)
    .bind(&input.auth_provider)
    .bind(input.is_impersonating)
    .bind(if input.is_impersonating {
        input.impersonator_user_workspace_id
    } else {
        None
    })
    .bind(if input.is_impersonating {
        input.impersonated_user_workspace_id
    } else {
        None
    })
    .bind(&input.user_agent)
    .bind(&input.ip_address)
    .bind(&input.country)
    .bind(&input.region)
    .bind(&input.city)
    .bind(expires_at)
    .bind(now)
    .execute(&state.core_pool)
    .await?;

    Ok((session_token, expires_at))
}

fn session_scope_matches_renewal(presented_payload: &Value, input: &CreateSessionInput) -> bool {
    let presented_user_id = presented_payload
        .get("userId")
        .or_else(|| presented_payload.get("sub"))
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok());

    let presented_type = presented_payload.get("type").and_then(Value::as_str);

    if presented_type != Some("ACCESS") {
        return presented_user_id == Some(input.user_id)
            && input.workspace_id.is_none()
            && !input.is_impersonating;
    }

    let parse_opt_uuid = |key: &str| -> Option<Uuid> {
        presented_payload
            .get(key)
            .and_then(Value::as_str)
            .and_then(|raw| Uuid::parse_str(raw).ok())
    };

    presented_user_id == Some(input.user_id)
        && parse_opt_uuid("workspaceId") == input.workspace_id
        && presented_payload
            .get("isImpersonating")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            == input.is_impersonating
        && parse_opt_uuid("userWorkspaceId") == input.user_workspace_id
        && parse_opt_uuid("impersonatorUserWorkspaceId") == input.impersonator_user_workspace_id
        && parse_opt_uuid("impersonatedUserWorkspaceId") == input.impersonated_user_workspace_id
}

pub async fn issue_session_for_token_pair(
    state: &AppState,
    context: &SessionRequestContext<'_>,
    token_pair: &IssuedTokenPair,
    origin: CreationOrigin,
) -> AuthResult<Vec<HeaderValue>> {
    if !is_request_allowed_to_receive_session_cookie(context.headers, &state.config) {
        return Err(AuthError::forbidden(SESSION_COOKIE_ISSUE_FORBIDDEN_MESSAGE));
    }

    let session_input = build_create_session_input(
        token_pair,
        context,
        state.config.is_cloudflare_geolocation_enabled,
    )
    .ok_or_else(|| {
        AuthError::unauthenticated("Cannot issue a session from an invalid token pair")
    })?;

    let presented_session_token = extract_session_token(context.headers, &state.config);

    if let Some(presented) = &presented_session_token {
        if origin == CreationOrigin::RenewalBridge {
            match resolve_session(state, presented).await {
                Ok(resolved) => {
                    if session_scope_matches_renewal(&resolved.payload, &session_input) {
                        return Ok(Vec::new());
                    }
                }
                Err(error) => {
                    if error.code != "UNAUTHENTICATED" && error.code != "SESSION_SUPERSEDED" {
                        return Err(error);
                    }
                }
            }
        }
    }

    let (session_token, expires_at) = create_session(state, &session_input).await?;

    let mut cookies = vec![session_cookie_value(
        &state.config,
        &session_token,
        expires_at,
    )];

    let Some(presented) = presented_session_token else {
        return Ok(cookies);
    };

    if session_input.is_impersonating {
        cookies.push(impersonator_cookie_value(&state.config, &presented));

        return Ok(cookies);
    }

    if let Err(error) =
        revoke_session_by_token(state, &presented, RevocationReason::Superseded).await
    {
        tracing::error!(%error, "failed to revoke the superseded session");
    }

    Ok(cookies)
}

pub async fn run_cleanup(state: &AppState) -> (u64, u64) {
    let threshold = Utc::now() - Duration::seconds(CLEANUP_RETENTION_SECS);

    let mut deleted_sessions = 0u64;
    let mut deleted_refresh_tokens = 0u64;

    loop {
        let deleted = sqlx::query(
            "DELETE FROM core.\"userSession\" \
             WHERE id IN (SELECT id FROM core.\"userSession\" \
             WHERE \"expiresAt\" < $1 OR \"revokedAt\" < $1 LIMIT $2)",
        )
        .bind(threshold)
        .bind(CLEANUP_BATCH_SIZE)
        .execute(&state.core_pool)
        .await
        .map(|result| result.rows_affected())
        .unwrap_or(0);

        deleted_sessions += deleted;

        if deleted < CLEANUP_BATCH_SIZE as u64 {
            break;
        }
    }

    loop {
        let deleted = sqlx::query(
            "DELETE FROM core.\"appToken\" \
             WHERE id IN (SELECT id FROM core.\"appToken\" \
             WHERE type = 'REFRESH_TOKEN' AND (\"expiresAt\" < $1 OR \"revokedAt\" < $1) LIMIT $2)",
        )
        .bind(threshold)
        .bind(CLEANUP_BATCH_SIZE)
        .execute(&state.core_pool)
        .await
        .map(|result| result.rows_affected())
        .unwrap_or(0);

        deleted_refresh_tokens += deleted;

        if deleted < CLEANUP_BATCH_SIZE as u64 {
            break;
        }
    }

    if deleted_sessions > 0 || deleted_refresh_tokens > 0 {
        tracing::info!(
            deleted_sessions,
            deleted_refresh_tokens,
            "deleted ended sessions and stale refresh tokens"
        );
    }

    (deleted_sessions, deleted_refresh_tokens)
}

pub async fn cleanup_loop(state: AppState) {
    loop {
        let now = chrono::Local::now();
        let tomorrow = now.date_naive() + chrono::Duration::days(1);

        let next_run = tomorrow
            .and_hms_opt(3, 0, 0)
            .and_then(|naive| chrono::Local.from_local_datetime(&naive).single())
            .unwrap_or_else(|| now + chrono::Duration::hours(24));

        let wait = (next_run - now)
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(3600));

        tokio::time::sleep(wait).await;

        run_cleanup(&state).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn headers(entries: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();

        for (name, value) in entries {
            map.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }

        map
    }

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
        }
    }

    #[test]
    fn generated_tokens_match_the_upstream_shape() {
        let token = generate_session_token();

        assert!(token.starts_with(USER_SESSION_TOKEN_PREFIX));
        assert_eq!(token.len(), USER_SESSION_TOKEN_PREFIX.len() + 43);
        assert!(token[USER_SESSION_TOKEN_PREFIX.len()..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    }

    #[test]
    fn token_hash_matches_the_node_sha256_hex_contract() {
        assert_eq!(
            hash_session_token("sess_abc"),
            "57c4db2f383cac904543e460b9e355bceb781431773ff1eb55b232622802d937"
        );
    }

    #[test]
    fn cookie_access_follows_the_opt_in_and_navigation_rules() {
        assert_eq!(resolve_cookie_access(&headers(&[])), CookieAccess::Denied);
        assert_eq!(
            resolve_cookie_access(&headers(&[(SESSION_COOKIE_OPT_IN_HEADER, "1")])),
            CookieAccess::OptedIn
        );
        assert_eq!(
            resolve_cookie_access(&headers(&[("sec-fetch-mode", "navigate")])),
            CookieAccess::Navigation
        );
        assert_eq!(
            resolve_cookie_access(&headers(&[("sec-fetch-mode", "cors")])),
            CookieAccess::Denied
        );
    }

    #[test]
    fn secure_deployment_is_https_or_same_site_none() {
        let mut settings = config();
        assert!(!is_secure_deployment(&settings));

        settings.server_url = "https://crm.example.com".to_string();
        assert!(is_secure_deployment(&settings));

        settings.server_url = "http://localhost:3000".to_string();
        settings.auth_cookie_same_site = "none".to_string();
        assert!(is_secure_deployment(&settings));
    }

    #[test]
    fn cookie_reads_prefer_the_secure_name_and_check_the_prefix() {
        let settings = config();

        let plain = headers(&[
            (SESSION_COOKIE_OPT_IN_HEADER, "1"),
            ("cookie", "twenty-session=sess_aaa; other=x"),
        ]);
        assert_eq!(
            extract_session_token(&plain, &settings).as_deref(),
            Some("sess_aaa")
        );

        let secure = headers(&[
            (SESSION_COOKIE_OPT_IN_HEADER, "1"),
            (
                "cookie",
                "__Host-twenty-session=sess_bbb; twenty-session=sess_aaa",
            ),
        ]);
        assert_eq!(
            extract_session_token(&secure, &settings).as_deref(),
            Some("sess_bbb")
        );

        let wrong_prefix = headers(&[
            (SESSION_COOKIE_OPT_IN_HEADER, "1"),
            ("cookie", "twenty-session=jwt-value"),
        ]);
        assert_eq!(extract_session_token(&wrong_prefix, &settings), None);

        let mut secure_settings = config();
        secure_settings.server_url = "https://crm.example.com".to_string();

        assert_eq!(
            extract_session_token(&plain, &secure_settings),
            None,
            "plain name is ignored on secure deployments"
        );

        let no_cookie = headers(&[(SESSION_COOKIE_OPT_IN_HEADER, "1")]);
        assert_eq!(extract_session_token(&no_cookie, &settings), None);

        let denied_access = headers(&[("cookie", "twenty-session=sess_aaa")]);
        assert_eq!(
            extract_session_token(&denied_access, &settings),
            None,
            "cookies are ignored without the opt-in or a navigation"
        );
    }

    #[test]
    fn has_session_cookie_matches_both_names() {
        let present = headers(&[
            (SESSION_COOKIE_OPT_IN_HEADER, "1"),
            ("cookie", "twenty-session=sess_aaa"),
        ]);
        assert!(has_session_cookie(&present));

        let absent = headers(&[(SESSION_COOKIE_OPT_IN_HEADER, "1"), ("cookie", "other=x")]);
        assert!(!has_session_cookie(&absent));

        let secure_named = headers(&[
            (SESSION_COOKIE_OPT_IN_HEADER, "1"),
            ("cookie", "__Host-twenty-session=sess_aaa"),
        ]);
        assert!(has_session_cookie(&secure_named));

        let denied = headers(&[("cookie", "twenty-session=sess_aaa")]);
        assert!(!has_session_cookie(&denied));
    }

    #[test]
    fn cookie_values_carry_the_upstream_attributes() {
        let settings = config();
        let expires = Utc::now();

        let set = session_cookie_value(&settings, "sess_aaa", expires);
        let value = set.to_str().unwrap();

        assert!(value.starts_with("twenty-session=sess_aaa; Path=/; Expires="));
        assert!(value.contains("HttpOnly"));
        assert!(value.contains("SameSite=Lax"));
        assert!(!value.contains("Secure"));

        let cleared = session_cookie_clear_values(&settings);
        assert_eq!(cleared.len(), 2);
        assert!(cleared[0]
            .to_str()
            .unwrap()
            .contains("Expires=Thu, 01 Jan 1970 00:00:00 GMT"));
        assert!(cleared[0]
            .to_str()
            .unwrap()
            .starts_with("__Host-twenty-session=;"));
        assert!(cleared[1].to_str().unwrap().starts_with("twenty-session=;"));

        let mut strict = config();
        strict.auth_cookie_same_site = "none".to_string();
        let secure_set = session_cookie_value(&strict, "sess_aaa", expires);
        let secure_value = secure_set.to_str().unwrap();

        assert!(secure_value.starts_with("__Host-twenty-session=sess_aaa"));
        assert!(secure_value.contains("SameSite=None"));
        assert!(secure_value.contains("Secure"));
    }

    #[test]
    fn comparable_origins_normalize_scheme_and_default_ports() {
        assert_eq!(
            to_comparable_origin("HTTP://Example.com:80"),
            Some("http://example.com".to_string())
        );
        assert_eq!(
            to_comparable_origin("https://crm.example.com:443"),
            Some("https://crm.example.com".to_string())
        );
        assert_eq!(
            to_comparable_origin("https://crm.example.com:3443"),
            Some("https://crm.example.com:3443".to_string())
        );
        assert_eq!(to_comparable_origin("ftp://crm.example.com"), None);
        assert_eq!(to_comparable_origin("null"), None);
        assert_eq!(
            to_comparable_origin("http://[::1]:3000"),
            Some("http://[::1]:3000".to_string())
        );
        assert_eq!(
            to_comparable_origin("http://[::1]"),
            Some("http://[::1]".to_string())
        );
    }

    #[test]
    fn loopback_detection_covers_the_upstream_hostnames() {
        assert!(is_loopback_hostname("localhost"));
        assert!(is_loopback_hostname("LOCALHOST."));
        assert!(is_loopback_hostname("::1"));
        assert!(is_loopback_hostname("127.0.0.1"));
        assert!(is_loopback_hostname("127.9.9.9"));
        assert!(is_loopback_hostname("::ffff:7f00:1"));
        assert!(!is_loopback_hostname("128.0.0.1"));
        assert!(!is_loopback_hostname("example.com"));
        assert!(!is_loopback_hostname("::ffff:2000:1"));
    }

    #[test]
    fn allowed_origins_derive_from_server_and_frontend_urls() {
        let settings = config();
        let allowed = allowed_credentialed_origins(&settings);

        assert_eq!(
            allowed,
            vec![
                "http://localhost:3000".to_string(),
                "http://localhost:3001".to_string()
            ]
        );
    }

    #[test]
    fn production_drops_derived_loopback_origins_but_keeps_explicit_ones() {
        let mut settings = config();
        settings.node_env = nudge_core::NodeEnv::Production;
        settings.auth_cookie_allowed_origins = "http://localhost:5173".to_string();

        let allowed = allowed_credentialed_origins(&settings);

        assert_eq!(allowed, vec!["http://localhost:5173".to_string()]);
    }

    #[test]
    fn request_origins_are_judged_against_base_url_and_allowlist() {
        let settings = config();

        let same_host = headers(&[
            ("host", "localhost:3000"),
            ("origin", "http://localhost:3000"),
        ]);
        assert!(is_request_origin_allowed(
            &same_host,
            &settings,
            "http://localhost:3000"
        ));

        let cross_host = headers(&[
            ("host", "localhost:3000"),
            ("origin", "http://evil.example"),
        ]);
        assert!(!is_request_origin_allowed(
            &cross_host,
            &settings,
            "http://evil.example"
        ));

        let mut explicit = config();
        explicit.auth_cookie_allowed_origins = "https://partner.example".to_string();
        assert!(is_request_origin_allowed(
            &cross_host,
            &explicit,
            "https://partner.example"
        ));

        let forwarded = headers(&[
            ("host", "internal:3000"),
            ("x-forwarded-proto", "https"),
            ("origin", "https://internal:3000"),
        ]);
        assert!(is_request_origin_allowed(
            &forwarded,
            &settings,
            "https://internal:3000"
        ));

        assert!(is_request_allowed_to_receive_session_cookie(
            &same_host, &settings
        ));
        assert!(!is_request_allowed_to_receive_session_cookie(
            &cross_host,
            &settings
        ));

        let no_origin = headers(&[("host", "localhost:3000")]);
        assert!(is_request_allowed_to_receive_session_cookie(
            &no_origin, &settings
        ));
    }

    #[test]
    fn session_payloads_mirror_the_upstream_jwt_shapes() {
        let workspace_session = CachedSession {
            session_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            workspace_id: Some(Uuid::new_v4()),
            user_workspace_id: Some(Uuid::new_v4()),
            auth_provider: "password".to_string(),
            is_impersonating: false,
            impersonator_user_workspace_id: None,
            impersonated_user_workspace_id: None,
            expires_at: Utc::now(),
            last_active_at: Utc::now(),
            authenticated_at: Utc::now(),
        };

        let payload = build_session_payload(&workspace_session).unwrap();

        assert_eq!(payload["type"], "ACCESS");
        assert_eq!(payload["isImpersonating"], false);
        assert_eq!(
            payload["workspaceId"],
            workspace_session.workspace_id.unwrap().to_string()
        );
        assert_eq!(
            payload["userWorkspaceId"],
            workspace_session.user_workspace_id.unwrap().to_string()
        );

        let agnostic = CachedSession {
            workspace_id: None,
            user_workspace_id: None,
            ..workspace_session.clone()
        };

        let agnostic_payload = build_session_payload(&agnostic).unwrap();

        assert_eq!(agnostic_payload["type"], "WORKSPACE_AGNOSTIC");
        assert!(agnostic_payload.get("workspaceId").is_none());

        let broken = CachedSession {
            workspace_id: Some(Uuid::new_v4()),
            user_workspace_id: None,
            ..workspace_session.clone()
        };

        assert!(build_session_payload(&broken).is_err());

        let impersonating = CachedSession {
            is_impersonating: true,
            impersonator_user_workspace_id: Some(Uuid::new_v4()),
            impersonated_user_workspace_id: Some(Uuid::new_v4()),
            ..workspace_session.clone()
        };

        let impersonating_payload = build_session_payload(&impersonating).unwrap();

        assert_eq!(impersonating_payload["isImpersonating"], true);
        assert_eq!(
            impersonating_payload["impersonatorUserWorkspaceId"],
            impersonating
                .impersonator_user_workspace_id
                .unwrap()
                .to_string()
        );
    }

    #[test]
    fn renewal_scope_match_requires_the_full_session_scope() {
        let user_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();

        let input = CreateSessionInput {
            user_id,
            workspace_id: Some(workspace_id),
            user_workspace_id: Some(user_workspace_id),
            auth_provider: "password".to_string(),
            is_impersonating: false,
            impersonator_user_workspace_id: None,
            impersonated_user_workspace_id: None,
            user_agent: None,
            ip_address: None,
            country: None,
            region: None,
            city: None,
        };

        let matching = serde_json::json!({
            "type": "ACCESS",
            "sub": user_id,
            "userId": user_id,
            "workspaceId": workspace_id.to_string(),
            "userWorkspaceId": user_workspace_id.to_string(),
            "isImpersonating": false,
        });

        assert!(session_scope_matches_renewal(&matching, &input));

        let other_workspace = serde_json::json!({
            "type": "ACCESS",
            "userId": user_id.to_string(),
            "workspaceId": Uuid::new_v4().to_string(),
            "userWorkspaceId": user_workspace_id.to_string(),
            "isImpersonating": false,
        });

        assert!(!session_scope_matches_renewal(&other_workspace, &input));

        let agnostic_presented = serde_json::json!({
            "type": "WORKSPACE_AGNOSTIC",
            "userId": user_id.to_string(),
        });

        assert!(!session_scope_matches_renewal(&agnostic_presented, &input));

        let mut agnostic_input = input.clone();
        agnostic_input.workspace_id = None;
        agnostic_input.user_workspace_id = None;

        assert!(session_scope_matches_renewal(
            &agnostic_presented,
            &agnostic_input
        ));
    }

    #[test]
    fn geolocation_reads_sanitized_cloudflare_headers() {
        let raw = headers(&[
            ("cf-ipcountry", "FR"),
            ("cf-region", "Ile-de-France"),
            ("cf-ipcity", "Paris"),
        ]);

        let (country, region, city) = cloudflare_geolocation(&raw);

        assert_eq!(country.as_deref(), Some("FR"));
        assert_eq!(region.as_deref(), Some("Ile-de-France"));
        assert_eq!(city.as_deref(), Some("Paris"));

        let unknown = headers(&[("cf-ipcountry", "XX")]);
        let (country, _, _) = cloudflare_geolocation(&unknown);
        assert_eq!(country, None);

        let tor = headers(&[("cf-ipcountry", "T1")]);
        let (country, _, _) = cloudflare_geolocation(&tor);
        assert_eq!(country, None);
    }

    #[test]
    fn revocation_reasons_use_the_upstream_strings() {
        let mut mapping = HashMap::new();

        for reason in [
            RevocationReason::UserSignOut,
            RevocationReason::Superseded,
            RevocationReason::PasswordChanged,
        ] {
            mapping.insert(reason.as_str().to_string(), reason);
        }

        assert_eq!(mapping.len(), 3);
        assert_eq!(mapping["USER_SIGN_OUT"], RevocationReason::UserSignOut);
        assert_eq!(mapping["SUPERSEDED"], RevocationReason::Superseded);
        assert_eq!(
            mapping["PASSWORD_CHANGED"],
            RevocationReason::PasswordChanged
        );
    }
}
