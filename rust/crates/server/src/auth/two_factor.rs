use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use axum::extract::ConnectInfo;
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base32::Alphabet;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use uuid::Uuid;

use super::crypto;
use super::error::{AuthError, AuthResult};
use super::jwt_service::TokenType;
use super::middleware::AuthContext;
use super::session::{self, CreationOrigin, SessionRequestContext};
use super::tokens::{self, AuthProvider, IssuedTokenPair};
use crate::state::AppState;

const TOTP_STEP_SECS: i64 = 30;
const TOTP_WINDOW: i64 = 1;
const TOTP_DIGITS: usize = 6;
const TOTP_SECRET_BYTES: usize = 10;
const PENDING_METHOD_REUSE_WINDOW_SECS: i64 = 60 * 60;
const USED_OTP_TTL_SECS: i64 = (2 * TOTP_WINDOW + 2) * TOTP_STEP_SECS;
const OTP_RATE_LIMIT_MAX: i64 = 10;
const OTP_RATE_LIMIT_WINDOW_SECS: i64 = 15 * 60;
const RECOVERY_CODE_REDEMPTION_RATE_LIMIT_MAX: i64 = 5;
const RECOVERY_CODE_REDEMPTION_RATE_LIMIT_WINDOW_SECS: i64 = 15 * 60;
const RECOVERY_CODE_ISSUANCE_RATE_LIMIT_MAX: i64 = 10;
const RECOVERY_CODE_ISSUANCE_RATE_LIMIT_WINDOW_SECS: i64 = 60 * 60;
const BACKUP_CODE_COUNT: usize = 10;
const RECOVERY_CODE_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const RECOVERY_CODE_LENGTH: usize = 20;
const RECOVERY_CODE_GROUP_SIZE: usize = 5;
const RECOVERY_CODE_EXPIRES_IN_SECS: i64 = 60 * 60;
const SESSION_REVOKED_REASON_2FA_RESET: &str = "TWO_FACTOR_AUTHENTICATION_RESET";
const ISSUER_PREFIX: &str = "Twenty";

const METHOD_ALREADY_PROVISIONED_MESSAGE: &str =
    "A two factor authentication method has already been set. Please delete it and try again.";
const IMPERSONATION_LOGIN_TOKEN_MESSAGE: &str =
    "This operation cannot be performed with an impersonation login token";
const WORKSPACE_MISMATCH_MESSAGE: &str = "Token is not valid for this workspace";
const RATE_LIMIT_MESSAGE: &str = "Rate limit reached. Please try again later.";
const STEP_UP_OTP_REQUIRED_MESSAGE: &str =
    "A two-factor authentication code is required for this action";
const STEP_UP_METHOD_REQUIRED_MESSAGE: &str =
    "Two-factor authentication must be enabled in the current workspace for this action";
const RECOVERY_SELF_TARGET_MESSAGE: &str = "A recovery code cannot be generated for yourself";
const RECOVERY_NOT_MEMBER_MESSAGE: &str = "The user is not a member of this workspace";
const RECOVERY_NO_METHOD_MESSAGE: &str =
    "The member has no verified two-factor authentication method";
const RECOVERY_ADMIN_TARGET_MESSAGE: &str =
    "Only a server administrator can manage recovery codes for a server administrator";
const RECOVERY_CONFLICT_MESSAGE: &str = "A recovery code was issued concurrently for this member";
const RECOVERY_ENROLLMENT_RESTRICTED_MESSAGE: &str =
    "Enrollment after a recovery must use the authenticator issued with the recovery code";
const METHOD_NOT_FOUND_MESSAGE: &str = "Two Factor Authentication Method not found.";
const MALFORMED_METHOD_MESSAGE: &str = "Malformed Two Factor Authentication Method object";
const METHOD_CHANGED_BEFORE_BACKUP_MESSAGE: &str =
    "The two-factor authentication method changed before backup codes were issued";
const METHOD_DELETE_NOT_FOUND_MESSAGE: &str = "Two-factor authentication method not found";
const METHOD_DELETE_FOREIGN_MESSAGE: &str =
    "You can only delete your own two-factor authentication methods";

#[derive(Debug, Clone, sqlx::FromRow)]
struct MethodRow {
    id: Uuid,
    secret: String,
    status: String,
    #[sqlx(rename = "createdAt")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ValidatedMethod {
    pub method_id: Uuid,
    pub has_completed_enrollment: bool,
}

pub struct RedemptionResult {
    pub tokens: Option<IssuedTokenPair>,
    pub provisioning_uri: Option<String>,
    pub is_two_factor_authentication_reset: bool,
}

pub struct OtpExchangeResult {
    pub tokens: IssuedTokenPair,
    pub email: String,
    pub user_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionInput {
    pub login_token: String,
    pub workspace_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyMethodInput {
    pub otp: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtpTokenInput {
    pub login_token: String,
    pub otp: String,
    pub workspace_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryTokenInput {
    pub login_token: String,
    pub recovery_code: String,
    pub workspace_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMethodInput {
    pub two_factor_authentication_method_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryTargetInput {
    pub user_id: Uuid,
    #[serde(default)]
    pub otp: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryStatusQuery {
    pub user_id: Uuid,
}

pub fn percent_encode_component(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());

    for byte in input.as_bytes() {
        let byte = *byte;

        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

pub fn issuer_for_display_name(display_name: Option<&str>) -> String {
    match display_name {
        Some(name) if !name.is_empty() => format!("{ISSUER_PREFIX} - {name}"),
        _ => ISSUER_PREFIX.to_string(),
    }
}

pub fn otpauth_uri(account_name: &str, issuer: &str, secret: &str) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&period=30&digits=6&algorithm=SHA1&issuer={}",
        percent_encode_component(issuer),
        percent_encode_component(account_name),
        secret,
        percent_encode_component(issuer)
    )
}

pub fn secret_from_raw_bytes(raw: &[u8]) -> String {
    let masked: Vec<u8> = raw.iter().map(|byte| byte & 0x7f).collect();

    base32::encode(Alphabet::Rfc4648 { padding: false }, &masked)
}

pub fn generate_totp_secret() -> String {
    let mut raw = [0u8; TOTP_SECRET_BYTES];
    rand::thread_rng().fill_bytes(&mut raw);

    secret_from_raw_bytes(&raw)
}

pub fn decode_totp_secret(secret: &str) -> Option<Vec<u8>> {
    let normalized = secret.trim().trim_end_matches('=').to_ascii_uppercase();

    base32::decode(Alphabet::Rfc4648 { padding: false }, &normalized)
}

fn otplib_hmac_key(secret_hex: &str) -> Vec<u8> {
    let min_len = 20usize;

    if secret_hex.len() < min_len {
        let repeated = secret_hex.repeat(min_len - secret_hex.len());

        return hex::decode(&repeated)
            .unwrap_or_default()
            .into_iter()
            .take(min_len)
            .collect();
    }

    hex::decode(secret_hex).unwrap_or_default()
}

fn totp_token_for_counter(secret_hex: &str, counter: i64, digits: usize) -> AuthResult<String> {
    let key = otplib_hmac_key(secret_hex);

    let mut mac = <Hmac<Sha1> as hmac::Mac>::new_from_slice(&key)
        .map_err(|error| AuthError::internal(error.to_string()))?;

    mac.update(counter.to_be_bytes().as_slice());

    let digest = mac.finalize().into_bytes();
    let offset = (digest[digest.len() - 1] & 0xf) as usize;

    if offset + 4 > digest.len() {
        return Err(AuthError::internal("totp digest too short"));
    }

    let binary = ((digest[offset] & 0x7f) as u32) << 24
        | (digest[offset + 1] as u32) << 16
        | (digest[offset + 2] as u32) << 8
        | digest[offset + 3] as u32;

    let divisor = 10u32.pow(digits as u32);

    Ok(format!("{:0width$}", binary % divisor, width = digits))
}

fn totp_token_at(secret: &str, epoch_ms: i64) -> AuthResult<String> {
    let decoded = decode_totp_secret(secret)
        .ok_or_else(|| AuthError::internal("invalid base32 totp secret"))?;

    let counter = epoch_ms / 1000 / TOTP_STEP_SECS;

    totp_token_for_counter(&hex::encode(decoded), counter, TOTP_DIGITS)
}

pub fn totp_check(secret: &str, token: &str, now_ms: i64) -> bool {
    let token = token.trim();

    if token.is_empty() || !token.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    for delta in [0, -TOTP_WINDOW, TOTP_WINDOW] {
        let epoch = now_ms + delta * TOTP_STEP_SECS * 1000;

        if let Ok(system_token) = totp_token_at(secret, epoch) {
            if system_token == token {
                return true;
            }
        }
    }

    false
}

pub fn generate_recovery_code() -> String {
    let mut bytes = [0u8; RECOVERY_CODE_LENGTH];
    rand::thread_rng().fill_bytes(&mut bytes);

    let symbols: String = bytes
        .iter()
        .map(|byte| {
            RECOVERY_CODE_ALPHABET
                .as_bytes()
                .get((*byte % RECOVERY_CODE_ALPHABET.len() as u8) as usize)
                .map(|index| *index as char)
                .unwrap_or('0')
        })
        .collect();

    let groups: Vec<String> = symbols
        .as_bytes()
        .chunks(RECOVERY_CODE_GROUP_SIZE)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect();

    groups.join("-")
}

pub fn normalize_recovery_code(code: &str) -> String {
    code.to_ascii_uppercase()
        .chars()
        .filter(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
        .collect()
}

pub fn hash_recovery_code(code: &str) -> String {
    hex::encode(Sha256::digest(normalize_recovery_code(code).as_bytes()))
}

pub fn backup_code_expires_at() -> AuthResult<DateTime<Utc>> {
    Utc.with_ymd_and_hms(9999, 12, 31, 23, 59, 59)
        .single()
        .map(|base| base + chrono::Duration::milliseconds(999))
        .ok_or_else(|| AuthError::internal("backup code expiry out of range"))
}

pub fn pending_method_is_reusable(
    status: &str,
    created_at: &DateTime<Utc>,
    now: DateTime<Utc>,
) -> bool {
    status == "PENDING"
        && now - *created_at < chrono::Duration::seconds(PENDING_METHOD_REUSE_WINDOW_SECS)
}

pub fn completes_enrollment(status_before: &str) -> bool {
    status_before == "PENDING"
}

pub fn used_otp_key(method_id: Uuid, token: &str) -> String {
    format!("two-factor-authentication-used-otp:{method_id}:{token}")
}

pub fn otp_rate_limit_key(user_id: Uuid, workspace_id: Uuid) -> String {
    format!("two-factor-authentication-otp:{user_id}:{workspace_id}")
}

pub fn recovery_code_issuance_rate_limit_key(actor_user_id: Uuid) -> String {
    format!("two-factor-authentication-recovery-code-issuance:{actor_user_id}")
}

pub fn recovery_code_redemption_rate_limit_key(user_workspace_id: Uuid) -> String {
    format!("two-factor-authentication-recovery-code:{user_workspace_id}")
}

fn invalid_otp() -> AuthError {
    AuthError::new(400, "INVALID_OTP", "Invalid OTP")
}

fn invalid_recovery_code() -> AuthError {
    AuthError::new(400, "INVALID_RECOVERY_CODE", "Invalid recovery code")
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn encryption_keys(config: &nudge_core::Config) -> Vec<String> {
    let mut keys = vec![match &config.encryption_key {
        Some(key) => key.clone(),
        None => config.app_secret.clone(),
    }];

    if let Some(fallback) = &config.fallback_encryption_key {
        keys.push(fallback.clone());
    }

    keys
}

fn decrypt_workspace_secret(
    config: &nudge_core::Config,
    envelope: &str,
    workspace_id: Uuid,
) -> AuthResult<String> {
    let rest = envelope
        .strip_prefix("enc:v2:")
        .ok_or_else(|| AuthError::internal("malformed two factor secret envelope"))?;

    let (key_id, payload_base64) = rest
        .split_once(':')
        .ok_or_else(|| AuthError::internal("malformed two factor secret envelope"))?;

    let sealed = BASE64
        .decode(payload_base64)
        .map_err(|error| AuthError::internal(error.to_string()))?;

    const NONCE_BYTES: usize = 12;
    const TAG_BYTES: usize = 16;

    if sealed.len() < NONCE_BYTES + TAG_BYTES {
        return Err(AuthError::internal("two factor secret payload too short"));
    }

    let (nonce_bytes, ciphertext) = sealed.split_at(NONCE_BYTES);
    let nonce = Nonce::from_slice(nonce_bytes);

    for raw_key in encryption_keys(config) {
        if crypto::encryption_key_id(&raw_key) != key_id {
            continue;
        }

        let key = crypto::derive_gcm_key(&raw_key, Some(workspace_id));
        let cipher = Aes256Gcm::new((&key).into());

        if let Ok(plaintext) = cipher.decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad: &[],
            },
        ) {
            return String::from_utf8(plaintext)
                .map_err(|error| AuthError::internal(error.to_string()));
        }
    }

    Err(AuthError::internal(
        "no encryption key matches the two factor secret envelope",
    ))
}

fn encrypt_workspace_secret(
    config: &nudge_core::Config,
    workspace_id: Uuid,
    plaintext: &str,
) -> AuthResult<String> {
    let primary = encryption_keys(config)
        .first()
        .cloned()
        .ok_or_else(|| AuthError::internal("no encryption key is configured"))?;

    Ok(crypto::encrypt_versioned_for_workspace(
        &primary,
        Some(workspace_id),
        plaintext,
    ))
}

async fn redis_connection(state: &AppState) -> AuthResult<redis::aio::MultiplexedConnection> {
    state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

async fn claim_used_otp(state: &AppState, method_id: Uuid, token: &str) -> AuthResult<bool> {
    let mut connection = redis_connection(state).await?;

    let acquired: Option<String> = redis::cmd("SET")
        .arg(used_otp_key(method_id, token))
        .arg("1")
        .arg("NX")
        .arg("PX")
        .arg(USED_OTP_TTL_SECS * 1000)
        .query_async(&mut connection)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    Ok(acquired.is_some())
}

async fn consume_rate_limit(
    state: &AppState,
    key: &str,
    max: i64,
    window_secs: i64,
) -> AuthResult<()> {
    let mut connection = redis_connection(state).await?;

    let count: i64 = redis::cmd("INCR")
        .arg(key)
        .query_async(&mut connection)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if count == 1 {
        redis::cmd("EXPIRE")
            .arg(key)
            .arg(window_secs)
            .query_async::<()>(&mut connection)
            .await
            .map_err(|error| AuthError::internal(error.to_string()))?;
    }

    if count > max {
        return Err(AuthError::new(429, "LIMIT_REACHED", RATE_LIMIT_MESSAGE));
    }

    Ok(())
}

async fn consume_login_token(state: &AppState, jwt_id: &str) -> AuthResult<()> {
    let mut connection = redis_connection(state).await?;

    let acquired: Option<String> = redis::cmd("SET")
        .arg(format!("login-token:consumed:{jwt_id}"))
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

async fn find_method_for_user_workspace(
    state: &AppState,
    user_workspace_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<Option<MethodRow>> {
    let row: Option<MethodRow> = sqlx::query_as(
        "SELECT id, secret, status::text AS status, \"createdAt\" \
         FROM core.\"twoFactorAuthenticationMethod\" \
         WHERE \"userWorkspaceId\" = $1 AND \"workspaceId\" = $2 \
         AND strategy = 'TOTP' AND \"deletedAt\" IS NULL",
    )
    .bind(user_workspace_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

async fn find_method_for_user(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<Option<MethodRow>> {
    let row: Option<MethodRow> = sqlx::query_as(
        "SELECT m.id, m.secret, m.status::text AS status, m.\"createdAt\" \
         FROM core.\"twoFactorAuthenticationMethod\" m \
         JOIN core.\"userWorkspace\" uw ON uw.id = m.\"userWorkspaceId\" \
         WHERE uw.\"userId\" = $1 AND uw.\"workspaceId\" = $2 \
         AND m.strategy = 'TOTP' AND m.\"deletedAt\" IS NULL AND uw.\"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row)
}

async fn workspace_issuer(state: &AppState, workspace_id: Uuid) -> AuthResult<String> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT \"displayName\" FROM core.\"workspace\" WHERE id = $1")
            .bind(workspace_id)
            .fetch_optional(&state.core_pool)
            .await?;

    let display_name = row.and_then(|(name,)| name);

    Ok(issuer_for_display_name(display_name.as_deref()))
}

async fn assert_enrollment_not_reserved_for_recovery(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<()> {
    let user_workspace = tokens::user_workspace_id(state, &user_id, &workspace_id).await;

    let user_workspace_id = match user_workspace {
        Ok(id) => id,
        Err(_) => return Ok(()),
    };

    let verified: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"twoFactorAuthenticationMethod\" \
         WHERE \"userWorkspaceId\" = $1 AND status = 'VERIFIED' AND \"deletedAt\" IS NULL \
         LIMIT 1",
    )
    .bind(user_workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    if verified.is_some() {
        return Ok(());
    }

    let reserved: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM core.\"appToken\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 \
         AND type = 'TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE' \
         AND \"deletedAt\" IS NOT NULL AND \"revokedAt\" IS NULL \
         AND \"createdAt\" >= (SELECT \"createdAt\" FROM core.\"userWorkspace\" WHERE id = $3))",
    )
    .bind(user_id)
    .bind(workspace_id)
    .bind(user_workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    if reserved.map(|(exists,)| exists).unwrap_or(false) {
        return Err(AuthError::new(
            403,
            "RECOVERY_ENROLLMENT_RESTRICTED",
            RECOVERY_ENROLLMENT_RESTRICTED_MESSAGE,
        ));
    }

    Ok(())
}

async fn upsert_pending_method(
    state: &AppState,
    user_workspace_id: Uuid,
    workspace_id: Uuid,
    encrypted_secret: &str,
) -> AuthResult<()> {
    sqlx::query(
        "INSERT INTO core.\"twoFactorAuthenticationMethod\" \
         (\"userWorkspaceId\", secret, status, strategy, \"workspaceId\") \
         VALUES ($1, $2, 'PENDING', 'TOTP', $3) \
         ON CONFLICT (\"userWorkspaceId\", strategy) \
         DO UPDATE SET secret = $2, status = 'PENDING', \"updatedAt\" = now()",
    )
    .bind(user_workspace_id)
    .bind(encrypted_secret)
    .bind(workspace_id)
    .execute(&state.core_pool)
    .await?;

    Ok(())
}

pub async fn initiate_provisioning(
    state: &AppState,
    user_id: Uuid,
    email: &str,
    workspace_id: Uuid,
) -> AuthResult<String> {
    let user_workspace_id = tokens::user_workspace_id(state, &user_id, &workspace_id).await?;

    let existing = find_method_for_user_workspace(state, user_workspace_id, workspace_id).await?;

    if let Some(method) = &existing {
        if method.status != "PENDING" {
            return Err(AuthError::new(
                403,
                "TWO_FACTOR_AUTHENTICATION_METHOD_ALREADY_PROVISIONED",
                METHOD_ALREADY_PROVISIONED_MESSAGE,
            ));
        }

        if pending_method_is_reusable(&method.status, &method.created_at, Utc::now()) {
            let secret = decrypt_workspace_secret(&state.config, &method.secret, workspace_id)?;
            let issuer = workspace_issuer(state, workspace_id).await?;

            return Ok(otpauth_uri(email, &issuer, &secret));
        }
    }

    let issuer = workspace_issuer(state, workspace_id).await?;
    let secret = generate_totp_secret();
    let encrypted = encrypt_workspace_secret(&state.config, workspace_id, &secret)?;

    upsert_pending_method(state, user_workspace_id, workspace_id, &encrypted).await?;

    Ok(otpauth_uri(email, &issuer, &secret))
}

async fn find_verified_method_id(
    executor: &mut sqlx::PgConnection,
    user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<Option<Uuid>> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT m.id FROM core.\"twoFactorAuthenticationMethod\" m \
         JOIN core.\"userWorkspace\" uw ON uw.id = m.\"userWorkspaceId\" \
         WHERE m.\"workspaceId\" = $1 AND m.status = 'VERIFIED' AND m.\"deletedAt\" IS NULL \
         AND uw.\"userId\" = $2 AND uw.\"workspaceId\" = $1 AND uw.\"deletedAt\" IS NULL \
         LIMIT 1",
    )
    .bind(workspace_id)
    .bind(user_id)
    .fetch_optional(executor)
    .await?;

    Ok(row.map(|(id,)| id))
}

async fn has_any_verified_method(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<bool> {
    let row: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM core.\"twoFactorAuthenticationMethod\" m \
         JOIN core.\"userWorkspace\" uw ON uw.id = m.\"userWorkspaceId\" \
         WHERE uw.\"userId\" = $1 AND uw.\"workspaceId\" = $2 \
         AND m.status = 'VERIFIED' AND m.\"deletedAt\" IS NULL AND uw.\"deletedAt\" IS NULL)",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row.map(|(exists,)| exists).unwrap_or(false))
}

async fn revoke_app_tokens(
    executor: &mut sqlx::PgConnection,
    user_id: Uuid,
    workspace_id: Uuid,
    token_type: &str,
    include_redeemed: bool,
) -> AuthResult<u64> {
    let filter = match include_redeemed {
        true => "",
        false => " AND \"deletedAt\" IS NULL",
    };

    let statement = format!(
        "UPDATE core.\"appToken\" SET \"revokedAt\" = now(), \"updatedAt\" = now() \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND type = $3 AND \"revokedAt\" IS NULL{filter}"
    );

    let affected = sqlx::query(&statement)
        .bind(user_id)
        .bind(workspace_id)
        .bind(token_type)
        .execute(executor)
        .await?
        .rows_affected();

    Ok(affected)
}

async fn revoke_recovery_codes(
    executor: &mut sqlx::PgConnection,
    user_id: Uuid,
    workspace_id: Uuid,
    include_redeemed: bool,
) -> AuthResult<u64> {
    revoke_app_tokens(
        executor,
        user_id,
        workspace_id,
        "TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE",
        include_redeemed,
    )
    .await
}

async fn acquire_user_authentication_lock(
    executor: &mut sqlx::PgConnection,
    user_id: Uuid,
) -> AuthResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("user-authentication:{user_id}"))
        .execute(executor)
        .await?;

    Ok(())
}

pub async fn validate_strategy(
    state: &AppState,
    user_id: Uuid,
    token: &str,
    workspace_id: Uuid,
) -> AuthResult<ValidatedMethod> {
    consume_rate_limit(
        state,
        &otp_rate_limit_key(user_id, workspace_id),
        OTP_RATE_LIMIT_MAX,
        OTP_RATE_LIMIT_WINDOW_SECS,
    )
    .await?;

    let method = find_method_for_user(state, user_id, workspace_id)
        .await?
        .ok_or_else(|| AuthError::new(403, "INVALID_CONFIGURATION", METHOD_NOT_FOUND_MESSAGE))?;

    if method.secret.is_empty() {
        return Err(AuthError::new(
            403,
            "MALFORMED_DATABASE_OBJECT",
            MALFORMED_METHOD_MESSAGE,
        ));
    }

    let secret = decrypt_workspace_secret(&state.config, &method.secret, workspace_id)?;

    if !totp_check(&secret, token, now_ms()) {
        return Err(invalid_otp());
    }

    let first_use = claim_used_otp(state, method.id, token).await?;

    if !first_use {
        return Err(invalid_otp());
    }

    let mut transaction = state.core_pool.begin().await?;

    acquire_user_authentication_lock(&mut transaction, user_id).await?;

    let affected = sqlx::query(
        "UPDATE core.\"twoFactorAuthenticationMethod\" \
         SET status = 'VERIFIED', \"updatedAt\" = now() \
         WHERE id = $1 AND \"workspaceId\" = $2 AND secret = $3",
    )
    .bind(method.id)
    .bind(workspace_id)
    .bind(&method.secret)
    .execute(&mut *transaction)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(AuthError::new(
            403,
            "INVALID_CONFIGURATION",
            METHOD_NOT_FOUND_MESSAGE,
        ));
    }

    revoke_recovery_codes(&mut transaction, user_id, workspace_id, true).await?;

    transaction.commit().await?;

    Ok(ValidatedMethod {
        method_id: method.id,
        has_completed_enrollment: completes_enrollment(&method.status),
    })
}

async fn assert_fresh_step_up(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
    otp: Option<&str>,
) -> AuthResult<ValidatedMethod> {
    let otp = otp
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .ok_or_else(|| {
            AuthError::new(
                400,
                "STEP_UP_AUTHENTICATION_REQUIRED",
                STEP_UP_OTP_REQUIRED_MESSAGE,
            )
        })?;

    let has_verified = has_any_verified_method(state, user_id, workspace_id).await?;

    if !has_verified {
        return Err(AuthError::new(
            400,
            "STEP_UP_AUTHENTICATION_REQUIRED",
            STEP_UP_METHOD_REQUIRED_MESSAGE,
        ));
    }

    validate_strategy(state, user_id, otp, workspace_id).await
}

async fn insert_backup_codes(
    executor: &mut sqlx::PgConnection,
    user_id: Uuid,
    workspace_id: Uuid,
    method_id: Uuid,
    codes: &[String],
) -> AuthResult<()> {
    let expires_at = backup_code_expires_at()?;

    for code in codes {
        sqlx::query(
            "INSERT INTO core.\"appToken\" \
             (\"userId\", \"workspaceId\", type, value, \"expiresAt\", context) \
             VALUES ($1, $2, 'TWO_FACTOR_AUTHENTICATION_BACKUP_CODE', $3, $4, $5)",
        )
        .bind(user_id)
        .bind(workspace_id)
        .bind(hash_recovery_code(code))
        .bind(expires_at)
        .bind(json!({ "twoFactorAuthenticationMethodId": method_id }))
        .execute(&mut *executor)
        .await?;
    }

    Ok(())
}

pub async fn issue_backup_codes(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
    method_id: Uuid,
) -> AuthResult<Vec<String>> {
    let codes: Vec<String> = (0..BACKUP_CODE_COUNT)
        .map(|_| generate_recovery_code())
        .collect();

    let mut transaction = state.core_pool.begin().await?;

    acquire_user_authentication_lock(&mut transaction, user_id).await?;

    let verified_method_id =
        find_verified_method_id(&mut transaction, user_id, workspace_id).await?;

    if verified_method_id != Some(method_id) {
        return Err(AuthError::new(
            403,
            "INVALID_CONFIGURATION",
            METHOD_CHANGED_BEFORE_BACKUP_MESSAGE,
        ));
    }

    revoke_app_tokens(
        &mut transaction,
        user_id,
        workspace_id,
        "TWO_FACTOR_AUTHENTICATION_BACKUP_CODE",
        false,
    )
    .await?;

    insert_backup_codes(&mut transaction, user_id, workspace_id, method_id, &codes).await?;

    transaction.commit().await?;

    Ok(codes)
}

pub async fn remaining_backup_code_count(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<i64> {
    let mut connection = state.core_pool.acquire().await?;

    let verified_method_id =
        find_verified_method_id(&mut connection, user_id, workspace_id).await?;

    let Some(method_id) = verified_method_id else {
        return Ok(0);
    };

    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT count(*) FROM core.\"appToken\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 \
         AND type = 'TWO_FACTOR_AUTHENTICATION_BACKUP_CODE' \
         AND \"deletedAt\" IS NULL AND \"revokedAt\" IS NULL \
         AND context ->> 'twoFactorAuthenticationMethodId' = $3",
    )
    .bind(user_id)
    .bind(workspace_id)
    .bind(method_id.to_string())
    .fetch_optional(&mut *connection)
    .await?;

    Ok(row.map(|(count,)| count).unwrap_or(0))
}

pub async fn delete_method(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
    method_id: Uuid,
) -> AuthResult<()> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT uw.\"userId\" FROM core.\"twoFactorAuthenticationMethod\" m \
         JOIN core.\"userWorkspace\" uw ON uw.id = m.\"userWorkspaceId\" \
         WHERE m.id = $1 AND m.\"workspaceId\" = $2",
    )
    .bind(method_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let owner_id = row
        .map(|(owner,)| owner)
        .ok_or_else(|| AuthError::invalid_input(METHOD_DELETE_NOT_FOUND_MESSAGE))?;

    if owner_id != user_id {
        return Err(AuthError::forbidden(METHOD_DELETE_FOREIGN_MESSAGE));
    }

    let mut transaction = state.core_pool.begin().await?;

    sqlx::query("DELETE FROM core.\"twoFactorAuthenticationMethod\" WHERE id = $1")
        .bind(method_id)
        .execute(&mut *transaction)
        .await?;

    revoke_app_tokens(
        &mut transaction,
        user_id,
        workspace_id,
        "TWO_FACTOR_AUTHENTICATION_BACKUP_CODE",
        false,
    )
    .await?;

    transaction.commit().await?;

    Ok(())
}

async fn consume_backup_code(
    executor: &mut sqlx::PgConnection,
    user_id: Uuid,
    workspace_id: Uuid,
    code: &str,
) -> AuthResult<bool> {
    let verified_method_id = find_verified_method_id(executor, user_id, workspace_id).await?;

    let Some(method_id) = verified_method_id else {
        return Ok(false);
    };

    let affected = sqlx::query(
        "UPDATE core.\"appToken\" SET \"deletedAt\" = now(), \"updatedAt\" = now() \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 \
         AND type = 'TWO_FACTOR_AUTHENTICATION_BACKUP_CODE' \
         AND \"deletedAt\" IS NULL AND \"revokedAt\" IS NULL \
         AND context ->> 'twoFactorAuthenticationMethodId' = $3 AND value = $4",
    )
    .bind(user_id)
    .bind(workspace_id)
    .bind(method_id.to_string())
    .bind(hash_recovery_code(code))
    .execute(executor)
    .await?
    .rows_affected();

    Ok(affected > 0)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .map(|database_error| database_error.code().as_deref() == Some("23505"))
        .unwrap_or(false)
}

struct RecoveryResetTarget {
    user_id: Uuid,
    email: String,
    user_workspace_id: Uuid,
    user_workspace_created_at: DateTime<Utc>,
}

async fn reset_with_recovery_code(
    state: &AppState,
    transaction: &mut sqlx::PgConnection,
    workspace_id: Uuid,
    recovery_code: &str,
    target: &RecoveryResetTarget,
) -> AuthResult<Option<(Option<String>, bool)>> {
    let user_id = target.user_id;
    let user_workspace_id = target.user_workspace_id;
    let user_workspace_created_at = target.user_workspace_created_at;
    let email = target.email.as_str();
    let row: Option<(bool, Option<String>)> = sqlx::query_as(
        "SELECT \"isTwoFactorAuthenticationEnforced\", \"displayName\" \
         FROM core.\"workspace\" WHERE id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(&mut *transaction)
    .await?;

    let (enforced, display_name) =
        row.ok_or_else(|| AuthError::workspace_not_found("Workspace not found"))?;

    let issuer = issuer_for_display_name(display_name.as_deref());

    let statement = match enforced {
        true => {
            "UPDATE core.\"appToken\" SET \"deletedAt\" = now(), \"updatedAt\" = now() \
             WHERE \"userId\" = $1 AND \"workspaceId\" = $2 \
             AND type = 'TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE' AND value = $3 \
             AND \"deletedAt\" IS NULL AND \"revokedAt\" IS NULL \
             AND \"expiresAt\" > $4 AND \"createdAt\" >= $5"
        }
        false => {
            "UPDATE core.\"appToken\" \
             SET \"deletedAt\" = now(), \"revokedAt\" = now(), \"updatedAt\" = now() \
             WHERE \"userId\" = $1 AND \"workspaceId\" = $2 \
             AND type = 'TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE' AND value = $3 \
             AND \"deletedAt\" IS NULL AND \"revokedAt\" IS NULL \
             AND \"expiresAt\" > $4 AND \"createdAt\" >= $5"
        }
    };

    let consumed = sqlx::query(statement)
        .bind(user_id)
        .bind(workspace_id)
        .bind(hash_recovery_code(recovery_code))
        .bind(Utc::now())
        .bind(user_workspace_created_at)
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    if consumed == 0 {
        return Ok(None);
    }

    sqlx::query(
        "DELETE FROM core.\"twoFactorAuthenticationMethod\" \
         WHERE \"workspaceId\" = $1 AND \"userWorkspaceId\" = $2",
    )
    .bind(workspace_id)
    .bind(user_workspace_id)
    .execute(&mut *transaction)
    .await?;

    revoke_app_tokens(
        &mut *transaction,
        user_id,
        workspace_id,
        "TWO_FACTOR_AUTHENTICATION_BACKUP_CODE",
        false,
    )
    .await?;

    sqlx::query(
        "UPDATE core.\"appToken\" \
         SET \"revokedAt\" = COALESCE(\"revokedAt\", now()), \"updatedAt\" = now(), \
             context = COALESCE(context, '{}'::jsonb) || jsonb_build_object('revokedReason', $1::text) \
         WHERE \"userId\" = $2 AND \"workspaceId\" = $3 \
         AND type = 'REFRESH_TOKEN' AND \"expiresAt\" > $4",
    )
    .bind(SESSION_REVOKED_REASON_2FA_RESET)
    .bind(user_id)
    .bind(workspace_id)
    .bind(Utc::now())
    .execute(&mut *transaction)
    .await?;

    if !enforced {
        return Ok(Some((None, true)));
    }

    let secret = generate_totp_secret();
    let encrypted = encrypt_workspace_secret(&state.config, workspace_id, &secret)?;

    sqlx::query(
        "INSERT INTO core.\"twoFactorAuthenticationMethod\" \
         (\"userWorkspaceId\", secret, status, strategy, \"workspaceId\") \
         VALUES ($1, $2, 'PENDING', 'TOTP', $3)",
    )
    .bind(user_workspace_id)
    .bind(&encrypted)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;

    Ok(Some((Some(otpauth_uri(email, &issuer, &secret)), true)))
}

async fn revoke_sessions_for_2fa_reset(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<()> {
    let token_hashes: Vec<(String,)> = sqlx::query_as(
        "UPDATE core.\"userSession\" \
         SET \"revokedAt\" = now(), \"revokedReason\" = $1, \"updatedAt\" = now() \
         WHERE \"userId\" = $2 AND \"workspaceId\" = $3 AND \"revokedAt\" IS NULL \
         RETURNING \"tokenHash\"",
    )
    .bind(SESSION_REVOKED_REASON_2FA_RESET)
    .bind(user_id)
    .bind(workspace_id)
    .fetch_all(&state.core_pool)
    .await?;

    if token_hashes.is_empty() {
        return Ok(());
    }

    let mut connection = redis_connection(state).await?;

    for (token_hash,) in token_hashes {
        redis::cmd("DEL")
            .arg(format!("engine:auth-session:{token_hash}"))
            .query_async::<()>(&mut connection)
            .await
            .map_err(|error| AuthError::internal(error.to_string()))?;
    }

    Ok(())
}

async fn resolve_login_token_subject(
    state: &AppState,
    login_token: &str,
    workspace_id: Uuid,
) -> AuthResult<(Uuid, String)> {
    let payload = state
        .jwt
        .verify_typed(login_token, TokenType::Login)
        .await?;

    let auth_provider = payload
        .get("authProvider")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if auth_provider == AuthProvider::Impersonation.as_str() {
        return Err(AuthError::forbidden(IMPERSONATION_LOGIN_TOKEN_MESSAGE));
    }

    let token_workspace = payload
        .get("workspaceId")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok());

    if token_workspace != Some(workspace_id) {
        return Err(AuthError::forbidden(WORKSPACE_MISMATCH_MESSAGE));
    }

    let email = payload
        .get("sub")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
        .to_string();

    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.\"user\" WHERE email = $1 AND \"deletedAt\" IS NULL")
            .bind(&email)
            .fetch_optional(&state.core_pool)
            .await?;

    let user_id = row
        .map(|(id,)| id)
        .ok_or_else(|| AuthError::user_not_found("User not found"))?;

    Ok((user_id, email))
}

pub async fn get_auth_tokens_from_otp(
    state: &AppState,
    login_token: &str,
    otp: &str,
    workspace_id: Uuid,
) -> AuthResult<OtpExchangeResult> {
    let (user_id, email) = resolve_login_token_subject(state, login_token, workspace_id).await?;

    validate_strategy(state, user_id, otp, workspace_id).await?;

    let payload = super::jwt_service::JwtService::decode_payload(login_token)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    let jwt_id = payload
        .get("jti")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
        .to_string();

    consume_login_token(state, &jwt_id).await?;

    let tokens =
        tokens::issue_token_pair(state, user_id, workspace_id, AuthProvider::Password).await?;

    Ok(OtpExchangeResult {
        tokens,
        email,
        user_id,
    })
}

pub async fn redeem_recovery_code(
    state: &AppState,
    login_token: &str,
    recovery_code: &str,
    workspace_id: Uuid,
) -> AuthResult<RedemptionResult> {
    let (user_id, email) = resolve_login_token_subject(state, login_token, workspace_id).await?;

    let row: Option<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, \"createdAt\" FROM core.\"userWorkspace\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 AND \"deletedAt\" IS NULL",
    )
    .bind(user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (user_workspace_id, user_workspace_created_at) =
        row.ok_or_else(|| AuthError::internal("User workspace not found"))?;

    consume_rate_limit(
        state,
        &recovery_code_redemption_rate_limit_key(user_workspace_id),
        RECOVERY_CODE_REDEMPTION_RATE_LIMIT_MAX,
        RECOVERY_CODE_REDEMPTION_RATE_LIMIT_WINDOW_SECS,
    )
    .await?;

    let mut transaction = state.core_pool.begin().await?;

    acquire_user_authentication_lock(&mut transaction, user_id).await?;

    let target = RecoveryResetTarget {
        user_id,
        email,
        user_workspace_id,
        user_workspace_created_at,
    };

    let backup_consumed =
        consume_backup_code(&mut transaction, user_id, workspace_id, recovery_code).await?;

    let redemption = if backup_consumed {
        Some((None, false))
    } else {
        reset_with_recovery_code(
            state,
            &mut transaction,
            workspace_id,
            recovery_code,
            &target,
        )
        .await?
    };

    transaction.commit().await?;

    let (provisioning_uri, is_reset) = redemption.ok_or_else(invalid_recovery_code)?;

    if is_reset {
        revoke_sessions_for_2fa_reset(state, user_id, workspace_id).await?;
    }

    if let Some(uri) = provisioning_uri {
        return Ok(RedemptionResult {
            tokens: None,
            provisioning_uri: Some(uri),
            is_two_factor_authentication_reset: true,
        });
    }

    let payload = super::jwt_service::JwtService::decode_payload(login_token)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    let jwt_id = payload
        .get("jti")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
        .to_string();

    consume_login_token(state, &jwt_id).await?;

    let tokens =
        tokens::issue_token_pair(state, user_id, workspace_id, AuthProvider::Password).await?;

    Ok(RedemptionResult {
        tokens: Some(tokens),
        provisioning_uri: None,
        is_two_factor_authentication_reset: is_reset,
    })
}

async fn assert_actor_has_security_flag(
    state: &AppState,
    actor_user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<()> {
    let user_workspace_id = tokens::user_workspace_id(state, &actor_user_id, &workspace_id).await?;

    let granted: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM core.\"roleTarget\" rt \
         JOIN core.\"role\" r ON r.id = rt.\"roleId\" \
         WHERE rt.\"userWorkspaceId\" = $1 AND rt.\"workspaceId\" = $2 \
         AND r.\"canUpdateAllSettings\" = true)",
    )
    .bind(user_workspace_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    if granted.map(|(flag,)| flag).unwrap_or(false) {
        return Ok(());
    }

    Err(AuthError::forbidden(
        "You do not have the permission to manage two-factor authentication recovery codes",
    ))
}

async fn fetch_admin_privileges(state: &AppState, user_id: Uuid) -> AuthResult<(bool, bool)> {
    let row: Option<(bool, bool)> = sqlx::query_as(
        "SELECT \"canImpersonate\", \"canAccessFullAdminPanel\" \
         FROM core.\"user\" WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.core_pool)
    .await?;

    row.ok_or_else(|| AuthError::user_not_found("User not found"))
}

fn has_admin_privileges(privileges: (bool, bool)) -> bool {
    privileges.0 || privileges.1
}

async fn is_awaiting_recovery_enrollment(
    state: &AppState,
    user_id: Uuid,
    workspace_id: Uuid,
    user_workspace_created_at: DateTime<Utc>,
) -> AuthResult<bool> {
    let row: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM core.\"appToken\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 \
         AND type = 'TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE' \
         AND \"deletedAt\" IS NOT NULL AND \"revokedAt\" IS NULL AND \"createdAt\" >= $3)",
    )
    .bind(user_id)
    .bind(workspace_id)
    .bind(user_workspace_created_at)
    .fetch_optional(&state.core_pool)
    .await?;

    Ok(row.map(|(exists,)| exists).unwrap_or(false))
}

async fn assert_manageable_recovery_target(
    state: &AppState,
    actor_user_id: Uuid,
    target_user_id: Uuid,
    workspace_id: Uuid,
) -> AuthResult<(Uuid, DateTime<Utc>)> {
    let row: Option<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT uw.id, uw.\"createdAt\" FROM core.\"userWorkspace\" uw \
         WHERE uw.\"userId\" = $1 AND uw.\"workspaceId\" = $2 AND uw.\"deletedAt\" IS NULL",
    )
    .bind(target_user_id)
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (target_user_workspace_id, target_user_workspace_created_at) = row.ok_or_else(|| {
        AuthError::new(
            403,
            "RECOVERY_CODE_TARGET_NOT_ALLOWED",
            RECOVERY_NOT_MEMBER_MESSAGE,
        )
    })?;

    let target_admin = has_admin_privileges(fetch_admin_privileges(state, target_user_id).await?);
    let actor_admin = has_admin_privileges(fetch_admin_privileges(state, actor_user_id).await?);

    if target_admin && !actor_admin {
        return Err(AuthError::new(
            403,
            "RECOVERY_CODE_TARGET_NOT_ALLOWED",
            RECOVERY_ADMIN_TARGET_MESSAGE,
        ));
    }

    Ok((target_user_workspace_id, target_user_workspace_created_at))
}

pub async fn generate_recovery_code_for_target(
    state: &AppState,
    actor_user_id: Uuid,
    workspace_id: Uuid,
    target_user_id: Uuid,
    otp: Option<&str>,
) -> AuthResult<(String, DateTime<Utc>)> {
    consume_rate_limit(
        state,
        &recovery_code_issuance_rate_limit_key(actor_user_id),
        RECOVERY_CODE_ISSUANCE_RATE_LIMIT_MAX,
        RECOVERY_CODE_ISSUANCE_RATE_LIMIT_WINDOW_SECS,
    )
    .await?;

    if actor_user_id == target_user_id {
        return Err(AuthError::new(
            403,
            "RECOVERY_CODE_TARGET_NOT_ALLOWED",
            RECOVERY_SELF_TARGET_MESSAGE,
        ));
    }

    assert_actor_has_security_flag(state, actor_user_id, workspace_id).await?;

    assert_fresh_step_up(state, actor_user_id, workspace_id, otp).await?;

    let (target_user_workspace_id, _) =
        assert_manageable_recovery_target(state, actor_user_id, target_user_id, workspace_id)
            .await?;

    let mut connection = state.core_pool.acquire().await?;

    let has_verified =
        find_verified_method_id(&mut connection, target_user_id, workspace_id).await?;

    let awaiting = if has_verified.is_none() {
        let created_at: Option<(DateTime<Utc>,)> =
            sqlx::query_as("SELECT \"createdAt\" FROM core.\"userWorkspace\" WHERE id = $1")
                .bind(target_user_workspace_id)
                .fetch_optional(&state.core_pool)
                .await?;

        match created_at {
            Some((created_at,)) => {
                is_awaiting_recovery_enrollment(state, target_user_id, workspace_id, created_at)
                    .await?
            }
            None => false,
        }
    } else {
        false
    };

    if has_verified.is_none() && !awaiting {
        return Err(AuthError::new(
            403,
            "RECOVERY_CODE_TARGET_NOT_ALLOWED",
            RECOVERY_NO_METHOD_MESSAGE,
        ));
    }

    revoke_recovery_codes(&mut connection, target_user_id, workspace_id, false).await?;

    let code = generate_recovery_code();
    let expires_at = Utc::now() + chrono::Duration::seconds(RECOVERY_CODE_EXPIRES_IN_SECS);

    let inserted = sqlx::query(
        "INSERT INTO core.\"appToken\" \
         (\"userId\", \"workspaceId\", type, value, \"expiresAt\", context) \
         VALUES ($1, $2, 'TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE', $3, $4, $5)",
    )
    .bind(target_user_id)
    .bind(workspace_id)
    .bind(hash_recovery_code(&code))
    .bind(expires_at)
    .bind(json!({ "issuedByUserId": actor_user_id }))
    .execute(&state.core_pool)
    .await;

    match inserted {
        Ok(_) => {}
        Err(error) if is_unique_violation(&error) => {
            return Err(AuthError::new(
                409,
                "RECOVERY_CODE_ISSUANCE_CONFLICT",
                RECOVERY_CONFLICT_MESSAGE,
            ));
        }
        Err(error) => return Err(error.into()),
    }

    Ok((code, expires_at))
}

pub async fn revoke_recovery_code_for_target(
    state: &AppState,
    actor_user_id: Uuid,
    workspace_id: Uuid,
    target_user_id: Uuid,
) -> AuthResult<bool> {
    assert_actor_has_security_flag(state, actor_user_id, workspace_id).await?;

    assert_manageable_recovery_target(state, actor_user_id, target_user_id, workspace_id).await?;

    let mut connection = state.core_pool.acquire().await?;

    let revoked =
        revoke_recovery_codes(&mut connection, target_user_id, workspace_id, false).await?;

    Ok(revoked > 0)
}

pub async fn recovery_status(
    state: &AppState,
    actor_user_id: Uuid,
    workspace_id: Uuid,
    target_user_id: Uuid,
) -> AuthResult<Value> {
    assert_actor_has_security_flag(state, actor_user_id, workspace_id).await?;

    let (target_user_workspace_id, target_user_workspace_created_at) =
        assert_manageable_recovery_target(state, actor_user_id, target_user_id, workspace_id)
            .await?;

    let mut connection = state.core_pool.acquire().await?;

    let has_verified_method =
        find_verified_method_id(&mut connection, target_user_id, workspace_id)
            .await?
            .is_some();

    let is_awaiting_enrollment = if has_verified_method {
        false
    } else {
        is_awaiting_recovery_enrollment(
            state,
            target_user_id,
            workspace_id,
            target_user_workspace_created_at,
        )
        .await?
    };

    let pending: Option<(DateTime<Utc>,)> = sqlx::query_as(
        "SELECT \"expiresAt\" FROM core.\"appToken\" \
         WHERE \"userId\" = $1 AND \"workspaceId\" = $2 \
         AND type = 'TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE' \
         AND \"deletedAt\" IS NULL AND \"revokedAt\" IS NULL AND \"expiresAt\" > $3 \
         LIMIT 1",
    )
    .bind(target_user_id)
    .bind(workspace_id)
    .bind(Utc::now())
    .fetch_optional(&state.core_pool)
    .await?;

    let _ = target_user_workspace_id;

    Ok(json!({
        "hasVerifiedTwoFactorAuthenticationMethod": has_verified_method,
        "isAwaitingRecoveryEnrollment": is_awaiting_enrollment,
        "pendingRecoveryCodeExpiresAt": pending.map(|(expires_at,)| expires_at.to_rfc3339()),
    }))
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

fn authenticated_user_id(context: &AuthContext) -> AuthResult<Uuid> {
    context
        .user_id
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))
}

async fn provision(
    State(state): State<AppState>,
    Json(input): Json<ProvisionInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let payload = state
        .jwt
        .verify_typed(&input.login_token, TokenType::Login)
        .await?;

    let auth_provider = payload
        .get("authProvider")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if auth_provider == AuthProvider::Impersonation.as_str() {
        return Err(AuthError::forbidden(IMPERSONATION_LOGIN_TOKEN_MESSAGE));
    }

    let token_workspace = payload
        .get("workspaceId")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok());

    if token_workspace != Some(input.workspace_id) {
        return Err(AuthError::forbidden(WORKSPACE_MISMATCH_MESSAGE));
    }

    let email = payload
        .get("sub")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
        .to_string();

    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM core.\"user\" WHERE email = $1 AND \"deletedAt\" IS NULL")
            .bind(&email)
            .fetch_optional(&state.core_pool)
            .await?;

    let user_id = row
        .map(|(id,)| id)
        .ok_or_else(|| AuthError::user_not_found("User not found"))?;

    assert_enrollment_not_reserved_for_recovery(&state, user_id, input.workspace_id).await?;

    let uri = initiate_provisioning(&state, user_id, &email, input.workspace_id).await?;

    assert_enrollment_not_reserved_for_recovery(&state, user_id, input.workspace_id).await?;

    Ok(Json(json!({ "uri": uri })))
}

async fn provision_for_user(
    State(state): State<AppState>,
    context: AuthContext,
) -> AuthResult<Json<serde_json::Value>> {
    let user_id = authenticated_user_id(&context)?;
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    assert_enrollment_not_reserved_for_recovery(&state, user_id, workspace_id).await?;

    let email_row: Option<(String,)> =
        sqlx::query_as("SELECT email::text FROM core.\"user\" WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.core_pool)
            .await?;

    let email = email_row
        .map(|(email,)| email)
        .ok_or_else(|| AuthError::user_not_found("User not found"))?;

    let uri = initiate_provisioning(&state, user_id, &email, workspace_id).await?;

    assert_enrollment_not_reserved_for_recovery(&state, user_id, workspace_id).await?;

    Ok(Json(json!({ "uri": uri })))
}

async fn verify_method(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<VerifyMethodInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let user_id = authenticated_user_id(&context)?;
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let validated = validate_strategy(&state, user_id, &input.otp, workspace_id).await?;

    let backup_codes = if validated.has_completed_enrollment {
        Some(issue_backup_codes(&state, user_id, workspace_id, validated.method_id).await?)
    } else {
        None
    };

    Ok(Json(json!({
        "success": true,
        "backupCodes": backup_codes,
        "twoFactorAuthenticationMethodId": validated.method_id,
    })))
}

async fn otp_tokens(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<OtpTokenInput>,
) -> AuthResult<Response> {
    let result =
        get_auth_tokens_from_otp(&state, &input.login_token, &input.otp, input.workspace_id)
            .await?;

    let context = session_context(&headers, Some(addr));
    let cookies = session::issue_session_for_token_pair(
        &state,
        &context,
        &result.tokens,
        CreationOrigin::SignIn,
    )
    .await?;

    Ok(response_with_cookies(
        StatusCode::OK,
        json!({
            "accessToken": result.tokens.access_token,
            "refreshToken": result.tokens.refresh_token,
            "user": { "id": result.user_id, "email": result.email },
        }),
        cookies,
    ))
}

async fn recovery_tokens(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<RecoveryTokenInput>,
) -> AuthResult<Response> {
    let result = redeem_recovery_code(
        &state,
        &input.login_token,
        &input.recovery_code,
        input.workspace_id,
    )
    .await?;

    let mut cookies = Vec::new();

    let body = match &result.tokens {
        Some(tokens) => {
            let context = session_context(&headers, Some(addr));
            cookies = session::issue_session_for_token_pair(
                &state,
                &context,
                tokens,
                CreationOrigin::SignIn,
            )
            .await?;

            let user_id = super::jwt_service::JwtService::decode_payload(&tokens.access_token)
                .and_then(|payload| payload.get("sub").and_then(Value::as_str).map(String::from));

            json!({
                "accessToken": tokens.access_token,
                "refreshToken": tokens.refresh_token,
                "user": user_id.map(|id| json!({ "id": id })),
                "provisioningUri": result.provisioning_uri,
                "isTwoFactorAuthenticationReset": result.is_two_factor_authentication_reset,
            })
        }
        None => json!({
            "tokens": Value::Null,
            "provisioningUri": result.provisioning_uri,
            "isTwoFactorAuthenticationReset": result.is_two_factor_authentication_reset,
        }),
    };

    Ok(response_with_cookies(StatusCode::OK, body, cookies))
}

async fn backup_codes_generate(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<VerifyMethodInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let user_id = authenticated_user_id(&context)?;
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let validated = assert_fresh_step_up(&state, user_id, workspace_id, Some(&input.otp)).await?;

    let backup_codes =
        issue_backup_codes(&state, user_id, workspace_id, validated.method_id).await?;

    Ok(Json(json!({ "backupCodes": backup_codes })))
}

async fn backup_codes_status(
    State(state): State<AppState>,
    context: AuthContext,
) -> AuthResult<Json<serde_json::Value>> {
    let user_id = authenticated_user_id(&context)?;
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let remaining = remaining_backup_code_count(&state, user_id, workspace_id).await?;

    Ok(Json(json!({ "remainingBackupCodeCount": remaining })))
}

async fn method_delete(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<DeleteMethodInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let user_id = authenticated_user_id(&context)?;
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    delete_method(
        &state,
        user_id,
        workspace_id,
        input.two_factor_authentication_method_id,
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}

async fn recovery_generate(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<RecoveryTargetInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let actor_user_id = authenticated_user_id(&context)?;
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let (recovery_code, expires_at) = generate_recovery_code_for_target(
        &state,
        actor_user_id,
        workspace_id,
        input.user_id,
        input.otp.as_deref(),
    )
    .await?;

    Ok(Json(json!({
        "recoveryCode": recovery_code,
        "expiresAt": expires_at.to_rfc3339(),
    })))
}

async fn recovery_revoke(
    State(state): State<AppState>,
    context: AuthContext,
    Json(input): Json<RecoveryTargetInput>,
) -> AuthResult<Json<serde_json::Value>> {
    let actor_user_id = authenticated_user_id(&context)?;
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let revoked =
        revoke_recovery_code_for_target(&state, actor_user_id, workspace_id, input.user_id).await?;

    Ok(Json(json!({ "success": revoked })))
}

async fn recovery_status_route(
    State(state): State<AppState>,
    context: AuthContext,
    axum::extract::Query(query): axum::extract::Query<RecoveryStatusQuery>,
) -> AuthResult<Json<serde_json::Value>> {
    let actor_user_id = authenticated_user_id(&context)?;
    let workspace_id = super::api_keys::workspace_id_from(&context)?;

    let status = recovery_status(&state, actor_user_id, workspace_id, query.user_id).await?;

    Ok(Json(status))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/otp/provision", post(provision))
        .route("/auth/otp/provision-user", post(provision_for_user))
        .route("/auth/otp/verify", post(verify_method))
        .route("/auth/otp/tokens", post(otp_tokens))
        .route("/auth/otp/recovery/tokens", post(recovery_tokens))
        .route("/auth/otp/backup-codes", post(backup_codes_generate))
        .route("/auth/otp/backup-codes/status", get(backup_codes_status))
        .route("/auth/otp/method/delete", post(method_delete))
        .route("/auth/otp/recovery/generate", post(recovery_generate))
        .route("/auth/otp/recovery/revoke", post(recovery_revoke))
        .route("/auth/otp/recovery/status", get(recovery_status_route))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE_VECTORS: &str = include_str!("../../../../tools/totp-test-vectors.json");

    fn vectors() -> serde_json::Value {
        serde_json::from_str(NODE_VECTORS).expect("node vectors parse as json")
    }

    fn fixed_secret() -> &'static str {
        "GEZDGNBVGY3TQOJQ"
    }

    #[test]
    fn rfc6238_eight_digit_vectors_match_appendix_b() {
        let expected = [
            (59i64, "94287082"),
            (1111111109, "07081804"),
            (1111111111, "14050471"),
            (1234567890, "89005924"),
            (2000000000, "69279037"),
            (20000000000, "65353130"),
        ];

        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

        for (seconds, code) in expected {
            let secret_hex = hex::encode(decode_totp_secret(secret).expect("decodes"));
            let counter = seconds / TOTP_STEP_SECS;

            assert_eq!(
                totp_token_for_counter(&secret_hex, counter, 8).unwrap(),
                code,
                "vector at t={seconds}"
            );
        }
    }

    #[test]
    fn totp_tokens_match_the_node_cross_check_vectors() {
        let vectors = vectors();

        let rfc = &vectors["rfc6238"];
        for (index, code) in rfc["sixDigits"]
            .as_array()
            .expect("sixDigits array")
            .iter()
            .enumerate()
        {
            let seconds = rfc["times"][index].as_i64().expect("time");
            assert_eq!(
                totp_token_at(rfc["secret"].as_str().expect("secret"), seconds * 1000).unwrap(),
                code.as_str().expect("code")
            );
        }

        let ten_byte = &vectors["tenByte"];
        assert_eq!(
            totp_token_at(ten_byte["secret"].as_str().expect("secret"), 1234567890000).unwrap(),
            ten_byte["at1234567890"].as_str().expect("vector")
        );
        assert_eq!(
            totp_token_at(ten_byte["secret"].as_str().expect("secret"), 1234567845000).unwrap(),
            ten_byte["at1234567845"].as_str().expect("vector")
        );
        assert_eq!(
            totp_token_at(ten_byte["secret"].as_str().expect("secret"), 1234567915000).unwrap(),
            ten_byte["at1234567915"].as_str().expect("vector")
        );

        let short = &vectors["shortSecret"];
        assert_eq!(
            totp_token_at(short["secret"].as_str().expect("secret"), 59000).unwrap(),
            short["at59"].as_str().expect("vector")
        );
        assert_eq!(
            hex::encode(otplib_hmac_key(&hex::encode(
                decode_totp_secret(short["secret"].as_str().expect("secret")).expect("decodes")
            ))),
            short["paddedKeyHex"].as_str().expect("padded key")
        );

        let long = &vectors["longSecret"];
        assert_eq!(
            totp_token_at(long["secret"].as_str().expect("secret"), 1234567890000).unwrap(),
            long["at1234567890"].as_str().expect("vector")
        );

        let generated = &vectors["generatedSecret"];
        let raw = hex::decode(generated["rawHex"].as_str().expect("raw hex")).expect("raw bytes");
        assert_eq!(
            secret_from_raw_bytes(&raw),
            generated["secret"].as_str().expect("secret")
        );
        assert_eq!(
            totp_token_at(generated["secret"].as_str().expect("secret"), 59000).unwrap(),
            generated["codeAt59"].as_str().expect("code")
        );
    }

    #[test]
    fn otp_window_accepts_one_step_each_way_and_rejects_two() {
        let vectors = vectors();
        let window = &vectors["window"];
        let secret = window["secret"].as_str().expect("secret");
        let base_ms = window["baseSeconds"].as_i64().expect("base") * 1000;

        assert!(totp_check(
            secret,
            window["at"].as_str().expect("at"),
            base_ms
        ));
        assert!(totp_check(
            secret,
            window["minusOneStep"].as_str().expect("minus"),
            base_ms
        ));
        assert!(totp_check(
            secret,
            window["plusOneStep"].as_str().expect("plus"),
            base_ms
        ));
        assert!(!totp_check(
            secret,
            window["minusTwoSteps"].as_str().expect("minus2"),
            base_ms
        ));
        assert!(!totp_check(
            secret,
            window["plusTwoSteps"].as_str().expect("plus2"),
            base_ms
        ));
    }

    #[test]
    fn otp_check_rejects_non_numeric_and_empty_tokens() {
        let secret = fixed_secret();
        let now = 1234567890000;

        assert!(!totp_check(secret, "", now));
        assert!(!totp_check(secret, "abcdef", now));
        assert!(!totp_check(secret, "12 45", now));
        assert!(!totp_check(secret, "1234567", now));
        assert!(totp_check(
            secret,
            totp_token_at(secret, now).unwrap().trim(),
            now
        ));
    }

    #[test]
    fn generated_secrets_match_the_upstream_shape() {
        for _ in 0..32 {
            let secret = generate_totp_secret();

            assert_eq!(secret.len(), 16);
            assert!(secret
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
            assert_eq!(decode_totp_secret(&secret).expect("decodes").len(), 10);
        }
    }

    #[test]
    fn decode_totp_secret_accepts_padding_and_lowercase() {
        assert_eq!(
            decode_totp_secret("gezdgnbvgy3tqojq"),
            decode_totp_secret("GEZDGNBVGY3TQOJQ")
        );
        assert_eq!(
            decode_totp_secret("GEZDGNBVGY3TQOJQ="),
            decode_totp_secret("GEZDGNBVGY3TQOJQ")
        );
        assert_eq!(decode_totp_secret("GEZDGNBVGY3TQOJ0"), None);
    }

    #[test]
    fn otpauth_uris_match_the_node_output() {
        let vectors = vectors();

        assert_eq!(
            otpauth_uri(
                "totp-vectors@nudge.test",
                "Twenty - Nudge HQ",
                fixed_secret()
            ),
            vectors["uri"].as_str().expect("uri")
        );
        assert_eq!(
            otpauth_uri("totp-vectors@nudge.test", "Twenty", fixed_secret()),
            vectors["uriPlainIssuer"].as_str().expect("plain uri")
        );
    }

    #[test]
    fn percent_encoding_matches_encodeuricomponent() {
        assert_eq!(percent_encode_component("a b"), "a%20b");
        assert_eq!(percent_encode_component("a@b"), "a%40b");
        assert_eq!(percent_encode_component("a:b"), "a%3Ab");
        assert_eq!(percent_encode_component("a!~*'()-_.b"), "a!~*'()-_.b");
        assert_eq!(percent_encode_component("é"), "%C3%A9");
        assert_eq!(percent_encode_component("Twenty - HQ"), "Twenty%20-%20HQ");
    }

    #[test]
    fn recovery_code_hashing_matches_the_node_contract() {
        let vectors = vectors();
        let expected = vectors["recoveryCodes"]["hash"].as_str().expect("hash");

        assert_eq!(hash_recovery_code("ABCDE-FGHIJ-KLMNP-QRSTV"), expected);
        assert_eq!(hash_recovery_code("abcde-fghij-klmnp-qrstv!@# "), expected);
        assert_eq!(hash_recovery_code("abcde-fghij-klmnp-qrstv"), expected);
        assert_eq!(
            normalize_recovery_code("abcde-fghij-klmnp-qrstv"),
            "ABCDE-FGHIJ-KLMNP-QRSTV".replace('-', "")
        );
    }

    #[test]
    fn generated_recovery_codes_have_the_upstream_shape() {
        for _ in 0..32 {
            let code = generate_recovery_code();
            let groups: Vec<&str> = code.split('-').collect();

            assert_eq!(
                code.len(),
                RECOVERY_CODE_LENGTH + (RECOVERY_CODE_LENGTH / RECOVERY_CODE_GROUP_SIZE) - 1
            );
            assert_eq!(groups.len(), 4);
            assert!(groups
                .iter()
                .all(|group| group.len() == RECOVERY_CODE_GROUP_SIZE
                    && group.chars().all(|c| RECOVERY_CODE_ALPHABET.contains(c))));
            assert_eq!(normalize_recovery_code(&code).len(), RECOVERY_CODE_LENGTH);
        }
    }

    #[test]
    fn pending_method_reuse_respects_the_one_hour_window() {
        let now = Utc::now();

        let fresh = now - chrono::Duration::seconds(60);
        let stale = now - chrono::Duration::seconds(PENDING_METHOD_REUSE_WINDOW_SECS + 1);
        let boundary = now - chrono::Duration::seconds(PENDING_METHOD_REUSE_WINDOW_SECS);

        assert!(pending_method_is_reusable("PENDING", &fresh, now));
        assert!(!pending_method_is_reusable("PENDING", &stale, now));
        assert!(!pending_method_is_reusable("PENDING", &boundary, now));
        assert!(!pending_method_is_reusable("VERIFIED", &fresh, now));
    }

    #[test]
    fn enrollment_completes_only_from_pending() {
        assert!(completes_enrollment("PENDING"));
        assert!(!completes_enrollment("VERIFIED"));
    }

    #[test]
    fn used_otp_ttl_is_two_minutes_like_the_upstream_constant() {
        assert_eq!(USED_OTP_TTL_SECS, 120);
    }

    #[test]
    fn backup_codes_never_expire_by_construction() {
        let expires_at = backup_code_expires_at().unwrap();

        assert_eq!(
            expires_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "9999-12-31T23:59:59.999Z"
        );
    }

    #[test]
    fn recovery_codes_are_redeemable_only_inside_their_window() {
        let now = Utc::now();
        let created = now - chrono::Duration::seconds(60);
        let workspace_created = now - chrono::Duration::hours(1);

        let live = now + chrono::Duration::seconds(RECOVERY_CODE_EXPIRES_IN_SECS);
        let expired = now - chrono::Duration::seconds(1);

        let redeemable = |expires_at: DateTime<Utc>, created_at: DateTime<Utc>| -> bool {
            expires_at > now && created_at >= workspace_created
        };

        assert!(redeemable(live, created));
        assert!(!redeemable(expired, created));

        let before_workspace = workspace_created - chrono::Duration::seconds(10);
        assert!(!redeemable(live, before_workspace));
        assert!(redeemable(live, workspace_created));
    }

    #[test]
    fn cache_and_rate_limit_keys_match_the_upstream_formats() {
        let method_id = Uuid::parse_str("7d87d9e8-0f4d-4f6e-9a5c-1b2c3d4e5f60").unwrap();
        let user_id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let workspace_id = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();

        assert_eq!(
            used_otp_key(method_id, "123456"),
            format!("two-factor-authentication-used-otp:{method_id}:123456")
        );
        assert_eq!(
            otp_rate_limit_key(user_id, workspace_id),
            format!("two-factor-authentication-otp:{user_id}:{workspace_id}")
        );
        assert_eq!(
            recovery_code_issuance_rate_limit_key(user_id),
            format!("two-factor-authentication-recovery-code-issuance:{user_id}")
        );
        assert_eq!(
            recovery_code_redemption_rate_limit_key(workspace_id),
            format!("two-factor-authentication-recovery-code:{workspace_id}")
        );
    }

    #[test]
    fn upstream_error_codes_and_statuses_are_used() {
        assert_eq!(invalid_otp().status, 400);
        assert_eq!(invalid_otp().code, "INVALID_OTP");
        assert_eq!(invalid_recovery_code().status, 400);
        assert_eq!(invalid_recovery_code().code, "INVALID_RECOVERY_CODE");
    }
}
