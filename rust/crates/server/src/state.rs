use std::sync::Arc;
use std::time::Duration;

use nudge_core::{Config, Result};
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub core_pool: PgPool,
    pub redis: Client,
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

        Ok(Self {
            config: Arc::new(config.clone()),
            core_pool,
            redis,
        })
    }
}
