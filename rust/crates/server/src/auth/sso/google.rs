use axum::extract::{Query, State};
use axum::response::Response;
use serde_json::Value;
use std::collections::HashMap;

use super::super::error::{AuthError, AuthResult};
use super::super::tokens::AuthProvider;
use super::config::GoogleSsoConfig;
use super::flow::{self, SocialProfile};
use super::state_store::SocialSsoState;
use super::{optional_query_string, redirect_response};
use crate::state::AppState;

const AUTHORIZATION_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USER_INFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

pub fn is_email_verified(raw: Option<&Value>) -> bool {
    match raw {
        Some(Value::Bool(true)) => true,
        Some(Value::String(raw)) => raw == "true",
        _ => false,
    }
}

pub fn profile_from_user_info(user_info: &Value) -> AuthResult<SocialProfile> {
    if !is_email_verified(user_info.get("email_verified")) {
        return Err(email_not_verified_error());
    }

    let email = user_info
        .get("email")
        .and_then(Value::as_str)
        .filter(|email| !email.is_empty())
        .ok_or_else(email_not_verified_error)?;

    Ok(SocialProfile {
        email: email.to_string(),
        first_name: optional_profile_field(user_info.get("given_name")),
        last_name: optional_profile_field(user_info.get("family_name")),
    })
}

pub fn email_not_verified_error() -> AuthError {
    AuthError::new(
        403,
        "EMAIL_NOT_VERIFIED",
        "Please verify your email address with Google",
    )
}

fn optional_profile_field(raw: Option<&Value>) -> Option<String> {
    raw.and_then(Value::as_str)
        .filter(|raw| !raw.is_empty())
        .map(String::from)
}

pub fn build_authorize_url(config: &GoogleSsoConfig, state: &SocialSsoState) -> String {
    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope=email%20profile&state={}&access_type=offline&prompt=select_account",
        AUTHORIZATION_URL,
        super::percent_encode(&config.client_id),
        super::percent_encode(&config.callback_url),
        super::percent_encode(&state.encode()),
    )
}

async fn http_client() -> AuthResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|error| AuthError::internal(error.to_string()))
}

async fn exchange_code_for_tokens(config: &GoogleSsoConfig, code: &str) -> AuthResult<String> {
    let response = http_client()
        .await?
        .post(TOKEN_URL)
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
                "Google OAuth access denied: {}",
                body.get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("token exchange failed")
            ),
        ));
    }

    body.get("access_token")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| AuthError::internal("Google token response has no access_token"))
}

async fn fetch_user_info(access_token: &str) -> AuthResult<Value> {
    let response = http_client()
        .await?
        .get(USER_INFO_URL)
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if !response.status().is_success() {
        return Err(AuthError::new(
            403,
            "OAUTH_ACCESS_DENIED",
            "Google OAuth access denied: the user info request failed",
        ));
    }

    response
        .json()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))
}

pub fn social_state_from_query(query: &HashMap<String, String>) -> SocialSsoState {
    SocialSsoState {
        workspace_invite_hash: optional_query_string(
            query.get("workspaceInviteHash").map(String::as_str),
        ),
        workspace_id: optional_query_string(query.get("workspaceId").map(String::as_str)),
        billing_checkout_session_state: optional_query_string(
            query.get("billingCheckoutSessionState").map(String::as_str),
        ),
        action: optional_query_string(query.get("action").map(String::as_str)),
        locale: optional_query_string(query.get("locale").map(String::as_str)),
        return_to_path: optional_query_string(query.get("returnToPath").map(String::as_str)),
        oauth_retry_count: None,
    }
}

pub async fn auth_start(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> AuthResult<Response> {
    let Some(config) = GoogleSsoConfig::from_env() else {
        return Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            "Google auth is not enabled",
        )));
    };

    let social_state = social_state_from_query(&query);

    Ok(redirect_response(&build_authorize_url(
        &config,
        &social_state,
    )))
}

pub async fn auth_redirect(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> AuthResult<Response> {
    let Some(config) = GoogleSsoConfig::from_env() else {
        return Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            "Google auth is not enabled",
        )));
    };

    let result = redirect_from_google(&state, &config, &query).await;

    match result {
        Ok(redirect) => Ok(redirect),
        Err(error) => Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            &error.messages.join(", "),
        ))),
    }
}

async fn redirect_from_google(
    state: &AppState,
    config: &GoogleSsoConfig,
    query: &HashMap<String, String>,
) -> AuthResult<Response> {
    if query.get("error").map(String::as_str) == Some("access_denied") {
        return Err(AuthError::new(
            403,
            "OAUTH_ACCESS_DENIED",
            "Google OAuth access denied",
        ));
    }

    let social_state = SocialSsoState::decode(query.get("state").map(String::as_str));

    let Some(code) = optional_query_string(query.get("code").map(String::as_str)) else {
        return Err(AuthError::invalid_input("Missing authorization code"));
    };

    let access_token = exchange_code_for_tokens(config, &code).await?;

    let user_info = fetch_user_info(&access_token).await?;

    let profile = profile_from_user_info(&user_info)?;

    let redirect =
        flow::sign_in_up_with_social_sso(state, profile, &social_state, AuthProvider::Google)
            .await?;

    Ok(redirect_response(&redirect))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_userinfo_maps_into_the_social_profile() {
        let fixture: Value = serde_json::from_str(
            r#"{
                "sub": "110169484474386276334",
                "name": "Jane Doe",
                "given_name": "Jane",
                "family_name": "Doe",
                "picture": "https://lh3.googleusercontent.com/a-/photo",
                "email": "jane@example.com",
                "email_verified": true,
                "locale": "en"
            }"#,
        )
        .unwrap();

        let profile = profile_from_user_info(&fixture).unwrap();

        assert_eq!(profile.email, "jane@example.com");
        assert_eq!(profile.first_name.as_deref(), Some("Jane"));
        assert_eq!(profile.last_name.as_deref(), Some("Doe"));
    }

    #[test]
    fn unverified_emails_are_rejected_with_email_not_verified() {
        let fixture: Value = serde_json::from_str(
            r#"{
                "sub": "1",
                "email": "jane@example.com",
                "email_verified": false
            }"#,
        )
        .unwrap();

        let error = profile_from_user_info(&fixture).unwrap_err();

        assert_eq!(error.code, "EMAIL_NOT_VERIFIED");
        assert_eq!(error.status, 403);
        assert_eq!(
            error.messages.join(", "),
            "Please verify your email address with Google"
        );
    }

    #[test]
    fn string_true_email_verified_is_accepted_like_the_passport_truthiness_check() {
        let fixture: Value = serde_json::from_str(
            r#"{
                "sub": "1",
                "email": "jane@example.com",
                "email_verified": "true"
            }"#,
        )
        .unwrap();

        let profile = profile_from_user_info(&fixture).unwrap();

        assert_eq!(profile.email, "jane@example.com");
    }

    #[test]
    fn missing_or_empty_emails_are_rejected() {
        let no_email: Value =
            serde_json::from_str(r#"{"sub": "1", "email_verified": true}"#).unwrap();

        assert_eq!(
            profile_from_user_info(&no_email).unwrap_err().code,
            "EMAIL_NOT_VERIFIED"
        );

        let empty_email: Value =
            serde_json::from_str(r#"{"sub": "1", "email": "", "email_verified": true}"#).unwrap();

        assert_eq!(
            profile_from_user_info(&empty_email).unwrap_err().code,
            "EMAIL_NOT_VERIFIED"
        );
    }

    #[test]
    fn authorize_urls_carry_client_id_callback_scope_and_encoded_state() {
        let config = GoogleSsoConfig {
            client_id: "google-client".to_string(),
            client_secret: "google-secret".to_string(),
            callback_url: "http://localhost:3000/auth/google/redirect".to_string(),
        };

        let state = SocialSsoState {
            workspace_invite_hash: Some("hash-1".to_string()),
            action: Some("sign-in".to_string()),
            ..SocialSsoState::default()
        };

        let url = build_authorize_url(&config, &state);

        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(url.contains("client_id=google-client"));
        assert!(
            url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fauth%2Fgoogle%2Fredirect")
        );
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=email%20profile"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=select_account"));
        assert!(url.contains(&format!(
            "state={}",
            super::super::percent_encode(&state.encode())
        )));
    }

    #[test]
    fn email_verified_rule_matches_the_upstream_truthiness() {
        assert!(is_email_verified(Some(&Value::Bool(true))));
        assert!(is_email_verified(Some(&Value::String("true".to_string()))));
        assert!(!is_email_verified(Some(&Value::Bool(false))));
        assert!(!is_email_verified(Some(&Value::String(
            "false".to_string()
        ))));
        assert!(!is_email_verified(Some(&Value::Null)));
        assert!(!is_email_verified(None));
    }
}
