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
        })
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
}
