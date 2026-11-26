use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::super::error::{AuthError, AuthResult};
use crate::state::AppState;

pub const STATE_TTL_SECS: u64 = 30 * 60;
pub const STATE_KEY_NAMESPACE: &str = "engine:session:sso";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SocialSsoState {
    #[serde(
        default,
        rename = "workspaceInviteHash",
        skip_serializing_if = "Option::is_none"
    )]
    pub workspace_invite_hash: Option<String>,
    #[serde(
        default,
        rename = "workspaceId",
        skip_serializing_if = "Option::is_none"
    )]
    pub workspace_id: Option<String>,
    #[serde(
        default,
        rename = "billingCheckoutSessionState",
        skip_serializing_if = "Option::is_none"
    )]
    pub billing_checkout_session_state: Option<String>,
    #[serde(default, rename = "action", skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, rename = "locale", skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(
        default,
        rename = "returnToPath",
        skip_serializing_if = "Option::is_none"
    )]
    pub return_to_path: Option<String>,
    #[serde(
        default,
        rename = "oauthRetryCount",
        skip_serializing_if = "Option::is_none"
    )]
    pub oauth_retry_count: Option<u32>,
}

impl SocialSsoState {
    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn decode(raw: Option<&str>) -> Self {
        raw.and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OidcStatePayload {
    #[serde(rename = "identityProviderId")]
    pub identity_provider_id: String,
    #[serde(rename = "codeVerifier")]
    pub code_verifier: String,
    #[serde(
        default,
        rename = "workspaceInviteHash",
        skip_serializing_if = "Option::is_none"
    )]
    pub workspace_invite_hash: Option<String>,
    #[serde(
        default,
        rename = "workspacePersonalInviteToken",
        skip_serializing_if = "Option::is_none"
    )]
    pub workspace_personal_invite_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SamlRelayStatePayload {
    #[serde(rename = "identityProviderId")]
    pub identity_provider_id: String,
    #[serde(
        default,
        rename = "workspaceInviteHash",
        skip_serializing_if = "Option::is_none"
    )]
    pub workspace_invite_hash: Option<String>,
    #[serde(default, rename = "token", skip_serializing_if = "Option::is_none")]
    pub workspace_personal_invite_token: Option<String>,
}

pub fn new_state_key() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);

    hex::encode(bytes)
}

pub fn generate_plain_exchange_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);

    hex::encode(bytes)
}

pub fn sha256_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());

    hex::encode(hasher.finalize())
}

pub fn pkce_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);

    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn pkce_code_challenge_s256(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());

    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

async fn redis_connection(state: &AppState) -> AuthResult<redis::aio::MultiplexedConnection> {
    state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

pub async fn save_state<T: Serialize>(state: &AppState, payload: &T) -> AuthResult<String> {
    let key = new_state_key();

    let serialized =
        serde_json::to_string(payload).map_err(|error| AuthError::internal(error.to_string()))?;

    redis::cmd("SETEX")
        .arg(format!("{STATE_KEY_NAMESPACE}:{key}"))
        .arg(STATE_TTL_SECS)
        .arg(serialized)
        .query_async::<()>(&mut redis_connection(state).await?)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    Ok(key)
}

pub async fn take_state<T: DeserializeOwned>(state: &AppState, key: &str) -> AuthResult<Option<T>> {
    let raw: Option<String> = redis::cmd("GETDEL")
        .arg(format!("{STATE_KEY_NAMESPACE}:{key}"))
        .query_async(&mut redis_connection(state).await?)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    match raw {
        Some(raw) => serde_json::from_str(&raw)
            .map(Some)
            .map_err(|error| AuthError::internal(error.to_string())),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn social_state_round_trips_with_upstream_key_names() {
        let encoded = SocialSsoState {
            workspace_invite_hash: Some("invite-hash-1".to_string()),
            workspace_id: Some("11111111-2222-3333-4444-555555555555".to_string()),
            billing_checkout_session_state: Some("cs_test_123".to_string()),
            action: Some("sign-in".to_string()),
            locale: Some("en".to_string()),
            return_to_path: Some("/settings/profile".to_string()),
            oauth_retry_count: None,
        }
        .encode();

        assert_eq!(
            encoded,
            "{\"workspaceInviteHash\":\"invite-hash-1\",\"workspaceId\":\"11111111-2222-3333-4444-555555555555\",\"billingCheckoutSessionState\":\"cs_test_123\",\"action\":\"sign-in\",\"locale\":\"en\",\"returnToPath\":\"/settings/profile\"}"
        );

        assert_eq!(
            SocialSsoState::decode(Some(&encoded)),
            SocialSsoState {
                workspace_invite_hash: Some("invite-hash-1".to_string()),
                workspace_id: Some("11111111-2222-3333-4444-555555555555".to_string()),
                billing_checkout_session_state: Some("cs_test_123".to_string()),
                action: Some("sign-in".to_string()),
                locale: Some("en".to_string()),
                return_to_path: Some("/settings/profile".to_string()),
                oauth_retry_count: None,
            }
        );
    }

    #[test]
    fn empty_state_fields_are_omitted_like_json_stringify() {
        let encoded = SocialSsoState {
            workspace_invite_hash: Some("abc".to_string()),
            ..SocialSsoState::default()
        }
        .encode();

        assert_eq!(encoded, "{\"workspaceInviteHash\":\"abc\"}");
    }

    #[test]
    fn missing_or_broken_state_decodes_to_the_default() {
        assert_eq!(SocialSsoState::decode(None), SocialSsoState::default());
        assert_eq!(
            SocialSsoState::decode(Some("not-json")),
            SocialSsoState::default()
        );
        assert_eq!(
            SocialSsoState::decode(Some("{}")),
            SocialSsoState::default()
        );
    }

    #[test]
    fn microsoft_retry_count_round_trips() {
        let encoded = SocialSsoState {
            oauth_retry_count: Some(2),
            ..SocialSsoState::default()
        }
        .encode();

        assert_eq!(encoded, "{\"oauthRetryCount\":2}");
        assert_eq!(
            SocialSsoState::decode(Some(&encoded)).oauth_retry_count,
            Some(2)
        );
    }

    #[test]
    fn oidc_state_payload_round_trips() {
        let payload = OidcStatePayload {
            identity_provider_id: "22222222-3333-4444-5555-666666666666".to_string(),
            code_verifier: "verifier-value".to_string(),
            workspace_invite_hash: Some("hash".to_string()),
            workspace_personal_invite_token: None,
        };

        let encoded = serde_json::to_string(&payload).unwrap();
        assert!(encoded.contains("\"identityProviderId\":\"22222222-3333-4444-5555-666666666666\""));

        let decoded: OidcStatePayload = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, payload);
    }

    #[test]
    fn saml_relay_state_payload_round_trips_with_the_upstream_token_key() {
        let payload = SamlRelayStatePayload {
            identity_provider_id: "33333333-4444-5555-6666-777777777777".to_string(),
            workspace_invite_hash: None,
            workspace_personal_invite_token: Some("personal-token".to_string()),
        };

        let encoded = serde_json::to_string(&payload).unwrap();

        assert_eq!(encoded, "{\"identityProviderId\":\"33333333-4444-5555-6666-777777777777\",\"token\":\"personal-token\"}");

        let decoded: SamlRelayStatePayload = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, payload);
    }

    #[test]
    fn state_keys_are_hex_of_32_random_bytes() {
        let key = new_state_key();

        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(key, new_state_key());
    }

    #[test]
    fn plain_exchange_tokens_are_64_hex_chars() {
        let token = generate_plain_exchange_token();

        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(token, generate_plain_exchange_token());
    }

    #[test]
    fn exchange_tokens_hash_like_the_node_sha256_hex_contract() {
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(sha256_hex("deadbeef"), {
            let mut hasher = Sha256::new();
            hasher.update(b"deadbeef");
            hex::encode(hasher.finalize())
        });
    }

    #[test]
    fn pkce_challenge_matches_the_rfc_7636_appendix_b_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

        assert_eq!(
            pkce_code_challenge_s256(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn pkce_verifiers_are_43_url_safe_chars() {
        let verifier = pkce_code_verifier();

        assert_eq!(verifier.len(), 43);
        assert!(verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(pkce_code_verifier(), verifier);
    }
}
