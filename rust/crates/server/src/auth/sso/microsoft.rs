use axum::extract::{Query, State};
use axum::response::Response;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use super::super::error::{AuthError, AuthResult};
use super::super::tokens::AuthProvider;
use super::config::MicrosoftSsoConfig;
use super::flow::{self, SocialProfile};
use super::google::social_state_from_query;
use super::state_store::SocialSsoState;
use super::{optional_query_string, redirect_response};
use crate::secure_http::client::SecureHttpClient;
use crate::state::AppState;

const AUTHORIZATION_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const USER_INFO_URL: &str = "https://graph.microsoft.com/v1.0/me";
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

pub fn profile_from_graph_user(user: &Value) -> AuthResult<SocialProfile> {
    let user_principal_name = user
        .get("userPrincipalName")
        .and_then(Value::as_str)
        .filter(|raw| !raw.is_empty())
        .ok_or_else(|| AuthError::invalid_input("User principal name not found"))?;

    Ok(SocialProfile {
        email: user_principal_name.to_string(),
        first_name: optional_profile_field(user.get("givenName")),
        last_name: optional_profile_field(user.get("surname")),
    })
}

fn optional_profile_field(raw: Option<&Value>) -> Option<String> {
    raw.and_then(Value::as_str)
        .filter(|raw| !raw.is_empty())
        .map(String::from)
}

pub fn build_authorize_url(config: &MicrosoftSsoConfig, state: &SocialSsoState) -> String {
    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope=User.Read&state={}&prompt=select_account",
        AUTHORIZATION_URL,
        super::percent_encode(&config.client_id),
        super::percent_encode(&config.callback_url),
        super::percent_encode(&state.encode()),
    )
}

async fn http_client() -> AuthResult<Arc<SecureHttpClient>> {
    SecureHttpClient::shared().map_err(|error| AuthError::internal(error.message()))
}

async fn exchange_code_for_tokens(config: &MicrosoftSsoConfig, code: &str) -> AuthResult<String> {
    let response = http_client()
        .await?
        .post(TOKEN_URL)
        .timeout(EXCHANGE_TIMEOUT)
        .form(&[
            ("code", code),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("redirect_uri", config.callback_url.as_str()),
            ("grant_type", "authorization_code"),
        ])
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
                "Microsoft OAuth access denied: {}",
                body.get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("token exchange failed")
            ),
        ));
    }

    body.get("access_token")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AuthError::internal("Microsoft token response has no access_token"))
}

async fn fetch_user_info(access_token: &str) -> AuthResult<Value> {
    let response = http_client()
        .await?
        .get(USER_INFO_URL)
        .timeout(EXCHANGE_TIMEOUT)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if !response.status().is_success() {
        return Err(AuthError::new(
            403,
            "OAUTH_ACCESS_DENIED",
            "Microsoft OAuth access denied: the user info request failed",
        ));
    }

    response
        .json()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

pub async fn auth_start(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> AuthResult<Response> {
    let Some(config) = MicrosoftSsoConfig::from_env() else {
        return Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            "Microsoft auth is not enabled",
        )));
    };

    let mut social_state = social_state_from_query(&query);

    if let Some(retry) = optional_query_string(query.get("oauthRetryCount").map(String::as_str)) {
        social_state.oauth_retry_count = retry.parse::<u32>().ok();
    }

    Ok(redirect_response(&build_authorize_url(
        &config,
        &social_state,
    )))
}

pub async fn auth_redirect(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> AuthResult<Response> {
    let Some(config) = MicrosoftSsoConfig::from_env() else {
        return Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            "Microsoft auth is not enabled",
        )));
    };

    let result = redirect_from_microsoft(&state, &config, &query).await;

    match result {
        Ok(redirect) => Ok(redirect),
        Err(error) => Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            &error.messages.join(", "),
        ))),
    }
}

async fn redirect_from_microsoft(
    state: &AppState,
    config: &MicrosoftSsoConfig,
    query: &HashMap<String, String>,
) -> AuthResult<Response> {
    let social_state = SocialSsoState::decode(query.get("state").map(String::as_str));

    let Some(code) = optional_query_string(query.get("code").map(String::as_str)) else {
        return Err(AuthError::invalid_input("Missing authorization code"));
    };

    let access_token = exchange_code_for_tokens(config, &code).await?;

    let user_info = fetch_user_info(&access_token).await?;

    let profile = profile_from_graph_user(&user_info)?;

    let redirect =
        flow::sign_in_up_with_social_sso(state, profile, &social_state, AuthProvider::Microsoft)
            .await?;

    Ok(redirect_response(&redirect))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn token_and_graph_fetches_run_on_the_guarded_client() {
        let client = http_client().await.expect("guarded microsoft client");

        assert!(client.policy().is_enforced());
        assert!(client
            .get("http://10.1.2.3/common/oauth2/v2.0/token")
            .send()
            .await
            .expect_err("rfc1918 token endpoint is refused")
            .is_blocked());
        assert!(client
            .get("http://[fd00::1]:443/v1.0/me")
            .send()
            .await
            .expect_err("unique local endpoint is refused")
            .is_blocked());
    }

    #[test]
    fn user_principal_name_becomes_the_email() {
        let fixture: Value = serde_json::from_str(
            r#"{
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "displayName": "Jane Doe",
                "givenName": "Jane",
                "surname": "Doe",
                "userPrincipalName": "jane@example.com",
                "mail": "jane.mail@example.com"
            }"#,
        )
        .unwrap();

        let profile = profile_from_graph_user(&fixture).unwrap();

        assert_eq!(profile.email, "jane@example.com");
        assert_eq!(profile.first_name.as_deref(), Some("Jane"));
        assert_eq!(profile.last_name.as_deref(), Some("Doe"));
    }

    #[test]
    fn missing_user_principal_names_are_invalid_input() {
        let fixture: Value =
            serde_json::from_str(r#"{"id": "1", "mail": "jane@example.com"}"#).unwrap();

        let error = profile_from_graph_user(&fixture).unwrap_err();

        assert_eq!(error.code, "INVALID_INPUT");
        assert_eq!(error.status, 400);
        assert_eq!(error.messages.join(", "), "User principal name not found");
    }

    #[test]
    fn empty_user_principal_names_are_invalid_input() {
        let fixture: Value = serde_json::from_str(
            r#"{"id": "1", "userPrincipalName": "", "mail": "jane@example.com"}"#,
        )
        .unwrap();

        let error = profile_from_graph_user(&fixture).unwrap_err();

        assert_eq!(error.messages.join(", "), "User principal name not found");
    }

    #[test]
    fn authorize_urls_target_the_common_tenant_with_user_read_scope() {
        let config = MicrosoftSsoConfig {
            client_id: "ms-client".to_string(),
            client_secret: "ms-secret".to_string(),
            callback_url: "http://localhost:3000/auth/microsoft/redirect".to_string(),
        };

        let state = SocialSsoState {
            oauth_retry_count: Some(1),
            ..SocialSsoState::default()
        };

        let url = build_authorize_url(&config, &state);

        assert!(url.starts_with("https://login.microsoftonline.com/common/oauth2/v2.0/authorize?"));
        assert!(url.contains("client_id=ms-client"));
        assert!(url.contains("scope=User.Read"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("prompt=select_account"));
        assert!(url.contains(&format!(
            "state={}",
            super::super::percent_encode(&state.encode())
        )));
    }
}
