use std::env;
use std::str::FromStr;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeEnv {
    Development,
    Production,
    Test,
}

impl NodeEnv {
    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            "test" => Ok(Self::Test),
            other => Err(Error::InvalidValue {
                key: "NODE_ENV",
                value: other.to_string(),
            }),
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub node_env: NodeEnv,
    pub node_port: u16,
    pub server_url: String,
    pub pg_database_url: String,
    pub pg_pool_max_connections: u32,
    pub redis_url: String,
    pub redis_cache_prefix: String,
    pub app_secret: String,
    pub encryption_key: Option<String>,
    pub fallback_encryption_key: Option<String>,
    pub access_token_expires_in_secs: u64,
    pub refresh_token_expires_in_secs: u64,
    pub login_token_expires_in_secs: u64,
    pub workspace_agnostic_token_expires_in_secs: u64,
    pub refresh_token_reuse_grace_period_secs: u64,
    pub frontend_url: String,
    pub auth_cookie_same_site: String,
    pub auth_cookie_allowed_origins: String,
    pub session_absolute_lifetime_secs: u64,
    pub session_idle_timeout_secs: u64,
    pub is_cloudflare_geolocation_enabled: bool,
    pub workspace_schema_ddl_locked: bool,
    pub search_ilike_fallback_timeout_ms: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            node_env: match var("NODE_ENV") {
                Some(raw) => NodeEnv::parse(&raw)?,
                None => NodeEnv::Production,
            },
            node_port: parse_or("NODE_PORT", 3000_u16)?,
            server_url: var_or("SERVER_URL", "http://localhost:3000"),
            pg_database_url: require("PG_DATABASE_URL")?,
            pg_pool_max_connections: parse_or("PG_POOL_MAX_CONNECTIONS", 10_u32)?,
            redis_url: require("REDIS_URL")?,
            redis_cache_prefix: var_or("REDIS_CACHE_PREFIX", ""),
            app_secret: require("APP_SECRET")?,
            encryption_key: var("ENCRYPTION_KEY"),
            fallback_encryption_key: var("FALLBACK_ENCRYPTION_KEY"),
            access_token_expires_in_secs: parse_duration_or("ACCESS_TOKEN_EXPIRES_IN", 30 * 60)?,
            refresh_token_expires_in_secs: parse_duration_or(
                "REFRESH_TOKEN_EXPIRES_IN",
                60 * 24 * 60 * 60,
            )?,
            login_token_expires_in_secs: parse_duration_or("LOGIN_TOKEN_EXPIRES_IN", 15 * 60)?,
            workspace_agnostic_token_expires_in_secs: parse_duration_or(
                "WORKSPACE_AGNOSTIC_TOKEN_EXPIRES_IN",
                30 * 60,
            )?,
            refresh_token_reuse_grace_period_secs: parse_duration_or(
                "REFRESH_TOKEN_REUSE_GRACE_PERIOD",
                60,
            )?,
            frontend_url: var("FRONTEND_URL").unwrap_or_else(|| {
                var("SERVER_URL").unwrap_or_else(|| "http://localhost:3000".to_string())
            }),
            auth_cookie_same_site: var_or("AUTH_COOKIE_SAME_SITE", "lax"),
            auth_cookie_allowed_origins: var_or("AUTH_COOKIE_ALLOWED_ORIGINS", ""),
            session_absolute_lifetime_secs: parse_duration_or(
                "SESSION_ABSOLUTE_LIFETIME",
                180 * 24 * 60 * 60,
            )?,
            session_idle_timeout_secs: parse_duration_or(
                "SESSION_IDLE_TIMEOUT",
                30 * 24 * 60 * 60,
            )?,
            is_cloudflare_geolocation_enabled: parse_bool_or(
                "IS_CLOUDFLARE_GEOLOCATION_ENABLED",
                false,
            )?,
            workspace_schema_ddl_locked: parse_bool_or("WORKSPACE_SCHEMA_DDL_LOCKED", false)?,
            search_ilike_fallback_timeout_ms: positive_u64_or(
                "SEARCH_ILIKE_FALLBACK_TIMEOUT_MS",
                2000,
            )?,
        })
    }
}

fn parse_bool_or(key: &'static str, default: bool) -> Result<bool> {
    match var(key) {
        Some(raw) => match raw.as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(Error::InvalidValue { key, value: raw }),
        },
        None => Ok(default),
    }
}

pub fn parse_duration(raw: &str) -> Option<u64> {
    let (digits, unit) = raw.split_at(raw.find(|c: char| !c.is_ascii_digit()).unwrap_or(raw.len()));

    let value = digits.parse::<u64>().ok()?;
    let multiplier = match unit {
        "s" | "" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        _ => return None,
    };

    value.checked_mul(multiplier)
}

fn parse_duration_or(key: &'static str, default_secs: u64) -> Result<u64> {
    match var(key) {
        Some(raw) => parse_duration(&raw).ok_or(Error::InvalidValue { key, value: raw }),
        None => Ok(default_secs),
    }
}

fn var(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn var_or(key: &str, default: &str) -> String {
    var(key).unwrap_or_else(|| default.to_string())
}

fn require(key: &'static str) -> Result<String> {
    var(key).ok_or(Error::MissingEnv(key))
}

fn parse_or<T>(key: &'static str, default: T) -> Result<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match var(key) {
        Some(raw) => raw.parse::<T>().map_err(|error| Error::InvalidValue {
            key,
            value: format!("{raw} ({error})"),
        }),
        None => Ok(default),
    }
}

fn positive_u64_or(key: &'static str, default: u64) -> Result<u64> {
    let value = parse_or(key, default)?;

    if value == 0 {
        return Err(Error::InvalidValue {
            key,
            value: value.to_string(),
        });
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_env_parses_the_three_known_modes() {
        assert_eq!(NodeEnv::parse("development").unwrap(), NodeEnv::Development);
        assert_eq!(NodeEnv::parse("production").unwrap(), NodeEnv::Production);
        assert_eq!(NodeEnv::parse("test").unwrap(), NodeEnv::Test);
    }

    #[test]
    fn node_env_rejects_an_unknown_mode() {
        let error = NodeEnv::parse("staging").unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidValue {
                key: "NODE_ENV",
                ..
            }
        ));
    }

    #[test]
    fn durations_match_the_ms_package_formats_the_original_uses() {
        assert_eq!(parse_duration("30m"), Some(30 * 60));
        assert_eq!(parse_duration("60d"), Some(60 * 24 * 60 * 60));
        assert_eq!(parse_duration("1h"), Some(3600));
        assert_eq!(parse_duration("5m"), Some(300));
        assert_eq!(parse_duration("900"), Some(900));
        assert_eq!(parse_duration("1y"), None);
        assert_eq!(parse_duration("m"), None);
    }
}
