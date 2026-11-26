fn env_optional(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_flag(key: &str) -> bool {
    env_optional(key).is_some_and(|value| value == "true" || value == "1")
}

#[derive(Debug, Clone)]
pub struct GoogleSsoConfig {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
}

#[derive(Debug, Clone)]
pub struct MicrosoftSsoConfig {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
}

impl GoogleSsoConfig {
    pub fn from_env() -> Option<Self> {
        if !env_flag("AUTH_GOOGLE_ENABLED") {
            return None;
        }

        Some(Self {
            client_id: env_optional("AUTH_GOOGLE_CLIENT_ID")?,
            client_secret: env_optional("AUTH_GOOGLE_CLIENT_SECRET")?,
            callback_url: env_optional("AUTH_GOOGLE_CALLBACK_URL")?,
        })
    }
}

impl MicrosoftSsoConfig {
    pub fn from_env() -> Option<Self> {
        if !env_flag("AUTH_MICROSOFT_ENABLED") {
            return None;
        }

        Some(Self {
            client_id: env_optional("AUTH_MICROSOFT_CLIENT_ID")?,
            client_secret: env_optional("AUTH_MICROSOFT_CLIENT_SECRET")?,
            callback_url: env_optional("AUTH_MICROSOFT_CALLBACK_URL")?,
        })
    }
}

pub fn saml_accepts_unverified_assertions() -> bool {
    env_flag("AUTH_SAML_ACCEPT_UNVERIFIED_ASSERTIONS")
}

pub fn is_email_verification_required() -> bool {
    env_flag("IS_EMAIL_VERIFICATION_REQUIRED")
}

pub fn is_multiworkspace_enabled() -> bool {
    env_flag("IS_MULTIWORKSPACE_ENABLED")
}

pub fn is_workspace_creation_limited_to_server_admins() -> bool {
    std::env::var("IS_WORKSPACE_CREATION_LIMITED_TO_SERVER_ADMINS")
        .ok()
        .map(|value| value.trim() != "false" && value.trim() != "0")
        .unwrap_or(true)
}
