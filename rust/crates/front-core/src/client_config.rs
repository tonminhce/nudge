use crate::client::ClientError;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ClientConfig {
    pub app_version: String,
    pub billing: Value,
    pub auth_providers: Value,
    pub sign_in_prefilled: bool,
    pub is_multi_workspace_enabled: bool,
    pub is_email_verification_required: bool,
    pub default_subdomain: String,
    pub front_domain: String,
    pub server_url: String,
    pub public_function_domain: String,
    pub support: Value,
    pub sentry: Value,
    pub captcha: Value,
    pub api: Value,
    pub onboarding: Option<Value>,
    pub is_attachment_preview_enabled: bool,
    pub analytics_enabled: bool,
    pub can_manage_feature_flags: bool,
    pub public_feature_flags: Vec<Value>,
    pub is_cookie_session_enabled: bool,
    pub is_workspace_schema_ddl_locked: bool,
    pub is_google_calendar_enabled: bool,
    pub is_google_messaging_enabled: bool,
    pub is_microsoft_calendar_enabled: bool,
    pub is_microsoft_messaging_enabled: bool,
    pub is_imap_smtp_caldav_enabled: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl ClientConfig {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ClientError> {
        serde_json::from_slice(bytes).map_err(ClientError::Json)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_client_config(base_url: &str) -> Result<ClientConfig, ClientError> {
    let url = format!("{}/client-config", base_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?;
    ClientConfig::from_bytes(&bytes)
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_client_config(base_url: &str) -> Result<ClientConfig, ClientError> {
    let url = format!("{}/client-config", base_url.trim_end_matches('/'));
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|error| ClientError::Transport(error.to_string()))?;
    let status = response.status();
    if !(200..300).contains(&status) {
        return Err(ClientError::Transport(format!("HTTP {status}")));
    }
    let text = response
        .text()
        .await
        .map_err(|error| ClientError::Transport(error.to_string()))?;
    ClientConfig::from_bytes(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_config_matches_the_live_reference_shape() {
        let config = ClientConfig::from_bytes(
            br#"{"appVersion":"v2.61.0","authProviders":{"password":true},"signInPrefilled":false,"isMultiWorkspaceEnabled":false,"isCookieSessionEnabled":true,"publicFeatureFlags":[{"key":"IS_AI_CHAT_INBOX_ENABLED"}]}"#,
        )
        .unwrap();
        assert_eq!(config.app_version, "v2.61.0");
        assert_eq!(config.auth_providers["password"], true);
        assert!(config.is_cookie_session_enabled);
        assert_eq!(
            config.public_feature_flags[0]["key"],
            "IS_AI_CHAT_INBOX_ENABLED"
        );
    }

    #[test]
    fn client_config_defaults_missing_keys_without_failing() {
        let config = ClientConfig::from_bytes(br#"{}"#).unwrap();
        assert!(config.app_version.is_empty());
        assert!(!config.is_multi_workspace_enabled);
        assert!(config.public_feature_flags.is_empty());
    }
}
