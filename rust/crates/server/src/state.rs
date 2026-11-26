use std::sync::Arc;
use std::time::Duration;

use nudge_core::{Config, Result};
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::auth::jwt_service::JwtService;
use crate::auth::signing_keys::SigningKeyManager;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub core_pool: PgPool,
    pub redis: Client,
    #[expect(dead_code, reason = "used by the key rotation surface in a later wave")]
    pub signing_keys: Arc<SigningKeyManager>,
    pub jwt: Arc<JwtService>,
}

impl AppState {
    pub async fn connect(config: &Config) -> Result<Self> {
        let core_pool = PgPoolOptions::new()
            .max_connections(config.pg_pool_max_connections)
            .acquire_timeout(Duration::from_secs(10))
            .after_connect(|conn, _metadata| {
                Box::pin(async move {
                    sqlx::query("SET search_path TO core, public")
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&config.pg_database_url)
            .await?;

        let redis = Client::open(config.redis_url.as_str())?;

        let mut raw_keys = vec![match &config.encryption_key {
            Some(key) => key.clone(),
            None => config.app_secret.clone(),
        }];

        if let Some(fallback) = &config.fallback_encryption_key {
            raw_keys.push(fallback.clone());
        }

        let signing_keys = Arc::new(SigningKeyManager::new(core_pool.clone(), raw_keys));
        let jwt = Arc::new(JwtService::new(
            config.app_secret.clone(),
            signing_keys.clone(),
        ));

        Ok(Self {
            config: Arc::new(config.clone()),
            core_pool,
            redis,
            signing_keys,
            jwt,
        })
    }
}
