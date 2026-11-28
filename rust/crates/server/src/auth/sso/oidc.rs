use axum::extract::{Path, Query, State};
use axum::response::Response;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::signature::Verifier;
use rsa::{BigUint, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

use super::super::error::{AuthError, AuthResult};
use super::flow::{self, idp_not_found_error};
use super::state_store::{
    pkce_code_challenge_s256, pkce_code_verifier, take_state, OidcStatePayload,
};
use super::{optional_query_string, redirect_response};
use crate::secure_http::client::SecureHttpClient;
use crate::state::AppState;

const OIDC_SCOPE: &str = "openid email profile";
const PREFERRED_ID_TOKEN_SIGNING_ALGORITHM: &str = "RS256";
const DISCOVERY_CACHE_TTL_SECS: u64 = 600;
const JWKS_CACHE_TTL_SECS: u64 = 600;
const ENDPOINT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryDocument {
    #[serde(rename = "issuer")]
    pub issuer: String,
    #[serde(rename = "authorization_endpoint")]
    pub authorization_endpoint: String,
    #[serde(rename = "token_endpoint")]
    pub token_endpoint: String,
    #[serde(rename = "userinfo_endpoint")]
    pub userinfo_endpoint: String,
    #[serde(rename = "jwks_uri")]
    pub jwks_uri: String,
    #[serde(
        default,
        rename = "id_token_signing_alg_values_supported",
        skip_serializing_if = "Option::is_none"
    )]
    pub id_token_signing_alg_values_supported: Option<Vec<String>>,
    #[serde(
        default,
        rename = "token_endpoint_auth_methods_supported",
        skip_serializing_if = "Option::is_none"
    )]
    pub token_endpoint_auth_methods_supported: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWebKey {
    #[serde(rename = "kty")]
    pub kty: String,
    #[serde(default, rename = "kid", skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>,
    #[serde(default, rename = "use", skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
    #[serde(default, rename = "n", skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
    #[serde(default, rename = "e", skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWebKeySet {
    #[serde(default, rename = "keys")]
    pub keys: Vec<JsonWebKey>,
}

pub fn resolve_id_token_signing_algorithm(supported: Option<&Vec<String>>) -> Option<String> {
    let supported = supported?;

    let signed: Vec<&String> = supported
        .iter()
        .filter(|algorithm| !algorithm.is_empty() && algorithm.as_str() != "none")
        .collect();

    if signed
        .iter()
        .any(|algorithm| algorithm == &PREFERRED_ID_TOKEN_SIGNING_ALGORITHM)
    {
        return Some(PREFERRED_ID_TOKEN_SIGNING_ALGORITHM.to_string());
    }

    signed.first().map(|algorithm| (*algorithm).clone())
}

pub fn well_known_configuration_url(issuer: &str) -> Option<String> {
    let (scheme, rest) = issuer.split_once("://")?;

    if scheme != "http" && scheme != "https" {
        return None;
    }

    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, Some(path)),
        None => (rest, None),
    };

    if authority.is_empty() {
        return None;
    }

    match path {
        None | Some("") => Some(format!(
            "{scheme}://{authority}/.well-known/openid-configuration"
        )),
        Some(path) => Some(format!(
            "{scheme}://{authority}/{}/.well-known/openid-configuration",
            path.trim_end_matches('/')
        )),
    }
}

pub fn parse_discovery_document(raw: &str) -> AuthResult<DiscoveryDocument> {
    let document: DiscoveryDocument = serde_json::from_str(raw)
        .map_err(|error| AuthError::new(400, "INVALID_ISSUER_URL", error.to_string()))?;

    for endpoint in [
        &document.issuer,
        &document.authorization_endpoint,
        &document.token_endpoint,
        &document.userinfo_endpoint,
        &document.jwks_uri,
    ] {
        if !endpoint.starts_with("https://") && !endpoint.starts_with("http://") {
            return Err(AuthError::new(
                400,
                "INVALID_ISSUER_URL",
                "Invalid issuer: the discovery document has invalid endpoints",
            ));
        }
    }

    Ok(document)
}

pub fn parse_jwks(raw: &str) -> AuthResult<JsonWebKeySet> {
    let set: JsonWebKeySet =
        serde_json::from_str(raw).map_err(|error| AuthError::internal(error.to_string()))?;

    let signing_keys: Vec<JsonWebKey> = set
        .keys
        .into_iter()
        .filter(|key| {
            key.kty == "RSA"
                && key.n.is_some()
                && key.e.is_some()
                && key.use_.as_deref().is_none_or(|use_| use_ == "sig")
        })
        .collect();

    Ok(JsonWebKeySet { keys: signing_keys })
}

fn discovery_cache() -> &'static Mutex<HashMap<String, (Instant, DiscoveryDocument)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (Instant, DiscoveryDocument)>>> = OnceLock::new();

    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn jwks_cache() -> &'static Mutex<HashMap<String, (Instant, JsonWebKeySet)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (Instant, JsonWebKeySet)>>> = OnceLock::new();

    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn http_client() -> AuthResult<Arc<SecureHttpClient>> {
    SecureHttpClient::shared().map_err(|error| AuthError::internal(error.message()))
}

async fn fetch_discovery_document(issuer: &str) -> AuthResult<DiscoveryDocument> {
    let url = well_known_configuration_url(issuer)
        .ok_or_else(|| idp_issuer_invalid_error("Invalid issuer: malformed issuer URL"))?;

    let cache = discovery_cache();

    if let Ok(guard) = cache.lock() {
        if let Some((fetched_at, document)) = guard.get(issuer) {
            if fetched_at.elapsed() < Duration::from_secs(DISCOVERY_CACHE_TTL_SECS) {
                return Ok(document.clone());
            }
        }
    }

    let response = http_client()
        .await?
        .get(&url)
        .timeout(ENDPOINT_TIMEOUT)
        .send()
        .await
        .map_err(|error| idp_issuer_invalid_error(&format!("Invalid issuer: {}", error)))?;

    if !response.status().is_success() {
        return Err(idp_issuer_invalid_error(&format!(
            "Invalid issuer: discovery responded with status {}",
            response.status()
        )));
    }

    let raw = response
        .text()
        .await
        .map_err(|error| idp_issuer_invalid_error(&format!("Invalid issuer: {}", error)))?;

    let document = parse_discovery_document(&raw)?;

    if document.issuer != issuer {
        return Err(idp_issuer_invalid_error(&format!(
            "Invalid issuer: the discovered issuer {} does not match {}",
            document.issuer, issuer
        )));
    }

    if let Ok(mut guard) = cache.lock() {
        guard.insert(issuer.to_string(), (Instant::now(), document.clone()));
    }

    Ok(document)
}

fn idp_issuer_invalid_error(reason: &str) -> AuthError {
    AuthError::new(400, "INVALID_ISSUER_URL", reason.to_string())
}

async fn fetch_jwks(jwks_uri: &str) -> AuthResult<JsonWebKeySet> {
    let cache = jwks_cache();

    if let Ok(guard) = cache.lock() {
        if let Some((fetched_at, set)) = guard.get(jwks_uri) {
            if fetched_at.elapsed() < Duration::from_secs(JWKS_CACHE_TTL_SECS) {
                return Ok(set.clone());
            }
        }
    }

    let response = http_client()
        .await?
        .get(jwks_uri)
        .timeout(ENDPOINT_TIMEOUT)
        .send()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if !response.status().is_success() {
        return Err(AuthError::internal("the OIDC JWKS request failed"));
    }

    let raw = response
        .text()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    let set = parse_jwks(&raw)?;

    if let Ok(mut guard) = cache.lock() {
        guard.insert(jwks_uri.to_string(), (Instant::now(), set.clone()));
    }

    Ok(set)
}

pub fn verify_id_token(
    id_token: &str,
    jwks: &JsonWebKeySet,
    expected_algorithm: &str,
    expected_issuer: &str,
    expected_client_id: &str,
    now: i64,
) -> AuthResult<Value> {
    let parts: Vec<&str> = id_token.split('.').collect();

    if parts.len() != 3 {
        return Err(AuthError::unauthenticated("Token invalid."));
    }

    let header: Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|_| AuthError::unauthenticated("Token invalid."))?,
    )
    .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    let algorithm = header
        .get("alg")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    if algorithm != expected_algorithm {
        return Err(AuthError::unauthenticated(
            "Token invalid.: the id token algorithm does not match the advertised one",
        ));
    }

    if algorithm != PREFERRED_ID_TOKEN_SIGNING_ALGORITHM {
        return Err(AuthError::unauthenticated(
            "Token invalid.: only RS256 id tokens are supported",
        ));
    }

    let kid = header.get("kid").and_then(Value::as_str);

    let key = match kid {
        Some(kid) => jwks
            .keys
            .iter()
            .find(|key| key.kid.as_deref() == Some(kid))
            .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?,
        None => {
            if jwks.keys.len() == 1 {
                jwks.keys
                    .first()
                    .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?
            } else {
                return Err(AuthError::unauthenticated("Token invalid."));
            }
        }
    };

    let modulus_bytes = URL_SAFE_NO_PAD
        .decode(key.n.as_deref().unwrap_or_default())
        .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    let exponent_bytes = URL_SAFE_NO_PAD
        .decode(key.e.as_deref().unwrap_or_default())
        .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    let public_key = RsaPublicKey::new(
        BigUint::from_bytes_be(&modulus_bytes),
        BigUint::from_bytes_be(&exponent_bytes),
    )
    .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    let verifying_key: VerifyingKey<Sha256> = VerifyingKey::new(public_key);

    let signature_bytes = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    let signature = Signature::try_from(signature_bytes.as_slice())
        .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    let signing_input = format!("{}.{}", parts[0], parts[1]);

    verifying_key
        .verify(signing_input.as_bytes(), &signature)
        .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    let payload: Value = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| AuthError::unauthenticated("Token invalid."))?,
    )
    .map_err(|_| AuthError::unauthenticated("Token invalid."))?;

    let issuer = payload
        .get("iss")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    if issuer != expected_issuer {
        return Err(AuthError::unauthenticated("Token invalid."));
    }

    let audience_matches = match payload.get("aud") {
        Some(Value::String(audience)) => audience == expected_client_id,
        Some(Value::Array(audiences)) => audiences
            .iter()
            .filter_map(Value::as_str)
            .any(|audience| audience == expected_client_id),
        _ => false,
    };

    if !audience_matches {
        return Err(AuthError::unauthenticated("Token invalid."));
    }

    let expires_at = payload
        .get("exp")
        .and_then(Value::as_i64)
        .ok_or_else(|| AuthError::unauthenticated("Token invalid."))?;

    if now >= expires_at {
        return Err(AuthError::unauthenticated("Token has expired."));
    }

    if payload.get("sub").and_then(Value::as_str).is_none() {
        return Err(AuthError::unauthenticated("Token invalid."));
    }

    Ok(payload)
}

struct IdentityProviderRow {
    issuer: String,
    client_id: String,
    client_secret: String,
}

async fn load_oidc_identity_provider(
    state: &AppState,
    identity_provider_id: &str,
) -> AuthResult<IdentityProviderRow> {
    let id = Uuid::parse_str(identity_provider_id).map_err(|_| idp_not_found_error())?;

    let row: Option<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT issuer, \"clientID\", \"clientSecret\", type::text \
         FROM core.\"workspaceSSOIdentityProvider\" WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (issuer, client_id, client_secret, idp_type) = row.ok_or_else(idp_not_found_error)?;

    if idp_type != "OIDC" {
        return Err(idp_not_found_error());
    }

    Ok(IdentityProviderRow {
        issuer,
        client_id: client_id.ok_or_else(idp_not_found_error)?,
        client_secret: client_secret.ok_or_else(idp_not_found_error)?,
    })
}

fn callback_url(state: &AppState) -> String {
    format!(
        "{}/auth/oidc/callback",
        state.config.server_url.trim_end_matches('/')
    )
}

pub async fn login(
    State(state): State<AppState>,
    Path(identity_provider_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> AuthResult<Response> {
    let result = login_redirect(&state, &identity_provider_id, &query).await;

    match result {
        Ok(redirect) => Ok(redirect),
        Err(error) => Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            &error.messages.join(", "),
        ))),
    }
}

async fn login_redirect(
    state: &AppState,
    identity_provider_id: &str,
    query: &HashMap<String, String>,
) -> AuthResult<Response> {
    let identity_provider = load_oidc_identity_provider(state, identity_provider_id).await?;

    let discovery = fetch_discovery_document(&identity_provider.issuer).await?;

    let code_verifier = pkce_code_verifier();

    let payload = OidcStatePayload {
        identity_provider_id: identity_provider_id.to_string(),
        code_verifier: code_verifier.clone(),
        workspace_invite_hash: optional_query_string(
            query.get("workspaceInviteHash").map(String::as_str),
        ),
        workspace_personal_invite_token: optional_query_string(
            query
                .get("workspacePersonalInviteToken")
                .map(String::as_str),
        ),
    };

    let state_key = super::state_store::save_state(state, &payload).await?;

    let authorization_url = format!(
        "{}?response_type=code&scope={}&client_id={}&redirect_uri={}&state={}&code_challenge={}&code_challenge_method=S256",
        discovery.authorization_endpoint,
        super::percent_encode(OIDC_SCOPE),
        super::percent_encode(&identity_provider.client_id),
        super::percent_encode(&callback_url(state)),
        super::percent_encode(&state_key),
        super::percent_encode(&pkce_code_challenge_s256(&code_verifier)),
    );

    Ok(redirect_response(&authorization_url))
}

pub async fn callback(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> AuthResult<Response> {
    let result = callback_redirect(&state, &query).await;

    match result {
        Ok(redirect) => Ok(redirect),
        Err(error) => Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            &error.messages.join(", "),
        ))),
    }
}

async fn callback_redirect(
    state: &AppState,
    query: &HashMap<String, String>,
) -> AuthResult<Response> {
    let Some(state_key) = optional_query_string(query.get("state").map(String::as_str)) else {
        return Err(AuthError::invalid_input("Invalid state"));
    };

    let payload: OidcStatePayload = take_state(state, &state_key)
        .await?
        .ok_or_else(|| AuthError::invalid_input("Invalid state"))?;

    if let Some(error) = optional_query_string(query.get("error").map(String::as_str)) {
        let description = optional_query_string(query.get("error_description").map(String::as_str))
            .unwrap_or(error);

        return Err(AuthError::new(403, "OAUTH_ACCESS_DENIED", description));
    }

    let identity_provider =
        load_oidc_identity_provider(state, &payload.identity_provider_id).await?;

    let discovery = fetch_discovery_document(&identity_provider.issuer).await?;

    let Some(code) = optional_query_string(query.get("code").map(String::as_str)) else {
        return Err(AuthError::invalid_input("Missing authorization code"));
    };

    let token_response = exchange_code_for_tokens(
        state,
        &identity_provider,
        &discovery,
        &code,
        &payload.code_verifier,
    )
    .await?;

    let id_token = token_response
        .get("id_token")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::invalid_input("the token response has no id_token"))?;

    let jwks = fetch_jwks(&discovery.jwks_uri).await?;

    let expected_algorithm = resolve_id_token_signing_algorithm(
        discovery.id_token_signing_alg_values_supported.as_ref(),
    )
    .unwrap_or_else(|| PREFERRED_ID_TOKEN_SIGNING_ALGORITHM.to_string());

    verify_id_token(
        id_token,
        &jwks,
        &expected_algorithm,
        &discovery.issuer,
        &identity_provider.client_id,
        chrono::Utc::now().timestamp(),
    )?;

    let access_token = token_response
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| AuthError::internal("the token response has no access_token"))?;

    let user_info = fetch_user_info(&discovery.userinfo_endpoint, access_token).await?;

    let email = user_info
        .get("email")
        .and_then(Value::as_str)
        .or_else(|| user_info.get("upn").and_then(Value::as_str))
        .filter(|email| !email.is_empty())
        .ok_or_else(|| {
            AuthError::new(
                400,
                "INVALID_DATA",
                "Email not found in identity provider payload",
            )
        })?
        .to_string();

    let redirect = flow::sso_idp_callback_redirect_url(
        state,
        &payload.identity_provider_id,
        &email,
        payload.workspace_invite_hash.as_deref(),
        payload.workspace_personal_invite_token.as_deref(),
    )
    .await?;

    Ok(redirect_response(&redirect))
}

async fn exchange_code_for_tokens(
    state: &AppState,
    identity_provider: &IdentityProviderRow,
    discovery: &DiscoveryDocument,
    code: &str,
    code_verifier: &str,
) -> AuthResult<Value> {
    let use_basic_auth = discovery
        .token_endpoint_auth_methods_supported
        .as_ref()
        .map(|methods| {
            methods.is_empty() || methods.iter().any(|method| method == "client_secret_basic")
        })
        .unwrap_or(true);

    let client = http_client().await?;
    let mut request = client
        .post(&discovery.token_endpoint)
        .timeout(ENDPOINT_TIMEOUT)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", callback_url(state).as_str()),
            ("code_verifier", code_verifier),
            ("client_id", identity_provider.client_id.as_str()),
        ]);

    if use_basic_auth {
        request = request.basic_auth(
            &identity_provider.client_id,
            &identity_provider.client_secret,
        );
    } else {
        request = request.form(&[("client_secret", identity_provider.client_secret.as_str())]);
    }

    let response = request
        .send()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if !status.is_success() {
        return Err(AuthError::new(
            403,
            "OAUTH_ACCESS_DENIED",
            format!(
                "OIDC OAuth access denied: {}",
                body.get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("token exchange failed")
            ),
        ));
    }

    Ok(body)
}

async fn fetch_user_info(userinfo_endpoint: &str, access_token: &str) -> AuthResult<Value> {
    let response = http_client()
        .await?
        .get(userinfo_endpoint)
        .timeout(ENDPOINT_TIMEOUT)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if !response.status().is_success() {
        return Err(AuthError::new(
            403,
            "OAUTH_ACCESS_DENIED",
            "OIDC OAuth access denied: the userinfo request failed",
        ));
    }

    response
        .json()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn discovery_refuses_an_issuer_on_the_loopback_interface() {
        let error = fetch_discovery_document("http://127.0.0.1:9456")
            .await
            .expect_err("a private issuer cannot be discovered");

        assert_eq!(error.code, "INVALID_ISSUER_URL");
        assert_eq!(
            error.messages,
            vec![
                "Invalid issuer: Request to internal IP address 127.0.0.1 is not allowed."
                    .to_string()
            ]
        );
    }

    #[tokio::test]
    async fn discovery_refuses_a_hostname_resolving_into_the_blocked_ranges() {
        let error = fetch_discovery_document("http://localhost:9456")
            .await
            .expect_err("a hostname answer inside the ranges is refused");

        assert_eq!(error.code, "INVALID_ISSUER_URL");
        assert!(
            error.messages[0].starts_with("Invalid issuer: Connection to internal IP address ")
                && error.messages[0].ends_with(" is not allowed."),
            "{:?}",
            error.messages
        );
    }

    #[tokio::test]
    async fn jwks_and_userinfo_endpoints_refuse_private_addresses() {
        let jwks_error = fetch_jwks("http://169.254.169.254/latest/jwks")
            .await
            .expect_err("the metadata address stays closed for jwks");

        assert_eq!(
            jwks_error.messages[0],
            "Request to internal IP address 169.254.169.254 is not allowed."
        );
        assert!(fetch_user_info("http://192.168.7.9/userinfo", "token")
            .await
            .expect_err("a private userinfo endpoint is refused")
            .messages[0]
            .contains("internal IP address 192.168.7.9 is not allowed"));
    }

    const DISCOVERY_FIXTURE: &str = r#"{
        "issuer": "https://idp.example.com",
        "authorization_endpoint": "https://idp.example.com/authorize",
        "token_endpoint": "https://idp.example.com/token",
        "userinfo_endpoint": "https://idp.example.com/userinfo",
        "jwks_uri": "https://idp.example.com/jwks",
        "id_token_signing_alg_values_supported": ["ES256", "RS256", "none"],
        "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post"]
    }"#;

    const JWKS_FIXTURE: &str = r#"{
        "keys": [
            {
                "kty": "EC",
                "crv": "P-256",
                "kid": "ec-key",
                "x": "abc",
                "y": "def"
            },
            {
                "kty": "RSA",
                "use": "enc",
                "kid": "enc-key",
                "n": "aaa",
                "e": "AQAB"
            },
            {
                "kty": "RSA",
                "use": "sig",
                "kid": "sig-key-1",
                "n": "b3NhaXNh",
                "e": "AQAB"
            },
            {
                "kty": "RSA",
                "kid": "sig-key-2",
                "n": "b3RoZXI",
                "e": "AQAB"
            }
        ]
    }"#;

    #[test]
    fn discovery_fixture_parses_into_the_endpoints_and_algorithms() {
        let document = parse_discovery_document(DISCOVERY_FIXTURE).unwrap();

        assert_eq!(document.issuer, "https://idp.example.com");
        assert_eq!(
            document.authorization_endpoint,
            "https://idp.example.com/authorize"
        );
        assert_eq!(document.token_endpoint, "https://idp.example.com/token");
        assert_eq!(
            document.userinfo_endpoint,
            "https://idp.example.com/userinfo"
        );
        assert_eq!(document.jwks_uri, "https://idp.example.com/jwks");
        assert_eq!(
            document.id_token_signing_alg_values_supported,
            Some(vec![
                "ES256".to_string(),
                "RS256".to_string(),
                "none".to_string()
            ])
        );
    }

    #[test]
    fn discovery_rejects_documents_with_missing_or_invalid_endpoints() {
        assert!(parse_discovery_document("{\"issuer\": \"https://x\"}").is_err());

        let ftp_issuer =
            DISCOVERY_FIXTURE.replace("https://idp.example.com\"", "ftp://idp.example.com\"");

        assert!(parse_discovery_document(&ftp_issuer).is_err());
    }

    #[test]
    fn algorithm_resolution_prefers_rs256_and_filters_none() {
        let fixture = parse_discovery_document(DISCOVERY_FIXTURE).unwrap();

        assert_eq!(
            resolve_id_token_signing_algorithm(
                fixture.id_token_signing_alg_values_supported.as_ref()
            ),
            Some("RS256".to_string())
        );

        let es_only = vec!["none".to_string(), "ES256".to_string()];
        assert_eq!(
            resolve_id_token_signing_algorithm(Some(&es_only)),
            Some("ES256".to_string())
        );

        let only_none = vec!["none".to_string()];
        assert_eq!(resolve_id_token_signing_algorithm(Some(&only_none)), None);

        assert_eq!(resolve_id_token_signing_algorithm(None), None);
    }

    #[test]
    fn jwks_fixture_keeps_only_rsa_signing_keys() {
        let set = parse_jwks(JWKS_FIXTURE).unwrap();

        assert_eq!(set.keys.len(), 2);
        assert_eq!(set.keys[0].kid.as_deref(), Some("sig-key-1"));
        assert_eq!(set.keys[0].use_.as_deref(), Some("sig"));
        assert_eq!(set.keys[1].kid.as_deref(), Some("sig-key-2"));
        assert_eq!(set.keys[1].use_.as_deref(), None);
    }

    #[test]
    fn well_known_urls_follow_the_path_aware_discovery_rule() {
        assert_eq!(
            well_known_configuration_url("https://idp.example.com"),
            Some("https://idp.example.com/.well-known/openid-configuration".to_string())
        );
        assert_eq!(
            well_known_configuration_url("https://idp.example.com/"),
            Some("https://idp.example.com/.well-known/openid-configuration".to_string())
        );
        assert_eq!(
            well_known_configuration_url("https://idp.example.com/realms/tenant"),
            Some(
                "https://idp.example.com/realms/tenant/.well-known/openid-configuration"
                    .to_string()
            )
        );
        assert_eq!(well_known_configuration_url("ftp://idp.example.com"), None);
        assert_eq!(well_known_configuration_url("https://"), None);
    }

    fn test_rsa_jwk_and_key() -> (JsonWebKey, rsa::RsaPrivateKey) {
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        use rsa::traits::PublicKeyParts;

        let mut rng = StdRng::seed_from_u64(42);
        let private_key = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let public_key = rsa::RsaPublicKey::from(&private_key);

        let modulus = public_key.n().to_bytes_be();
        let exponent = public_key.e().to_bytes_be();

        let jwk = JsonWebKey {
            kty: "RSA".to_string(),
            kid: Some("test-key".to_string()),
            use_: Some("sig".to_string()),
            n: Some(URL_SAFE_NO_PAD.encode(&modulus)),
            e: Some(URL_SAFE_NO_PAD.encode(&exponent)),
        };

        (jwk, private_key)
    }

    fn sign_id_token(private_key: &rsa::RsaPrivateKey, header: &Value, payload: &Value) -> String {
        use rsa::pkcs1v15::SigningKey;
        use rsa::signature::{SignatureEncoding, Signer};

        let encoded_header = URL_SAFE_NO_PAD.encode(header.to_string());
        let encoded_payload = URL_SAFE_NO_PAD.encode(payload.to_string());
        let signing_input = format!("{encoded_header}.{encoded_payload}");

        let signing_key: SigningKey<Sha256> = SigningKey::new(private_key.clone());
        let signature = signing_key.sign(signing_input.as_bytes());

        format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        )
    }

    #[test]
    fn rs256_id_tokens_verify_against_the_jwks_and_check_iss_aud_exp() {
        let (jwk, private_key) = test_rsa_jwk_and_key();
        let jwks = JsonWebKeySet { keys: vec![jwk] };

        let header = serde_json::json!({ "alg": "RS256", "kid": "test-key", "typ": "JWT" });
        let payload = serde_json::json!({
            "iss": "https://idp.example.com",
            "aud": "client-1",
            "sub": "user-1",
            "exp": 4102444800i64,
        });

        let token = sign_id_token(&private_key, &header, &payload);

        let verified = verify_id_token(
            &token,
            &jwks,
            "RS256",
            "https://idp.example.com",
            "client-1",
            1700000000,
        )
        .unwrap();

        assert_eq!(verified.get("sub").and_then(Value::as_str), Some("user-1"));

        let multi_audience = serde_json::json!({
            "iss": "https://idp.example.com",
            "aud": ["other-client", "client-1"],
            "sub": "user-2",
            "exp": 4102444800i64,
        });

        let multi_token = sign_id_token(&private_key, &header, &multi_audience);

        let verified = verify_id_token(
            &multi_token,
            &jwks,
            "RS256",
            "https://idp.example.com",
            "client-1",
            1700000000,
        )
        .unwrap();

        assert_eq!(verified.get("sub").and_then(Value::as_str), Some("user-2"));

        let wrong_issuer = serde_json::json!({
            "iss": "https://evil.example.com",
            "aud": "client-1",
            "sub": "user-3",
            "exp": 4102444800i64,
        });

        assert!(verify_id_token(
            &sign_id_token(&private_key, &header, &wrong_issuer),
            &jwks,
            "RS256",
            "https://idp.example.com",
            "client-1",
            1700000000,
        )
        .is_err());

        let wrong_audience = serde_json::json!({
            "iss": "https://idp.example.com",
            "aud": "other-client",
            "sub": "user-4",
            "exp": 4102444800i64,
        });

        assert!(verify_id_token(
            &sign_id_token(&private_key, &header, &wrong_audience),
            &jwks,
            "RS256",
            "https://idp.example.com",
            "client-1",
            1700000000,
        )
        .is_err());

        let expired = serde_json::json!({
            "iss": "https://idp.example.com",
            "aud": "client-1",
            "sub": "user-5",
            "exp": 1600000000i64,
        });

        assert!(verify_id_token(
            &sign_id_token(&private_key, &header, &expired),
            &jwks,
            "RS256",
            "https://idp.example.com",
            "client-1",
            1700000000,
        )
        .is_err());

        let unknown_kid_header =
            serde_json::json!({ "alg": "RS256", "kid": "other-key", "typ": "JWT" });

        assert!(verify_id_token(
            &sign_id_token(&private_key, &unknown_kid_header, &payload),
            &jwks,
            "RS256",
            "https://idp.example.com",
            "client-1",
            1700000000,
        )
        .is_err());

        let tampered = {
            let mut parts: Vec<String> = token.split('.').map(String::from).collect();
            let mut payload_value = payload.clone();
            payload_value["sub"] = serde_json::json!("attacker");
            parts[1] = URL_SAFE_NO_PAD.encode(payload_value.to_string());
            parts.join(".")
        };

        assert!(verify_id_token(
            &tampered,
            &jwks,
            "RS256",
            "https://idp.example.com",
            "client-1",
            1700000000,
        )
        .is_err());

        let hs256_header = serde_json::json!({ "alg": "HS256", "typ": "JWT" });

        assert!(verify_id_token(
            &sign_id_token(&private_key, &hs256_header, &payload),
            &jwks,
            "RS256",
            "https://idp.example.com",
            "client-1",
            1700000000,
        )
        .is_err());
    }
}
