use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub status: &'static str,
    pub version: &'static str,
    pub server_url: String,
    pub checks: HealthChecks,
}

#[derive(Debug, Serialize)]
pub struct HealthChecks {
    pub database: &'static str,
    pub redis: &'static str,
}

pub async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<HealthReport>) {
    let database = check_database(&state).await;
    let redis = check_redis(&state).await;
    let healthy = database == "up" && redis == "up";

    let status = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(HealthReport {
            status: if healthy { "ok" } else { "degraded" },
            version: env!("CARGO_PKG_VERSION"),
            server_url: state.config.server_url.clone(),
            checks: HealthChecks { database, redis },
        }),
    )
}

async fn check_database(state: &AppState) -> &'static str {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.core_pool)
        .await
    {
        Ok(_) => "up",
        Err(error) => {
            tracing::error!(%error, "database health check failed");
            "down"
        }
    }
}

async fn check_redis(state: &AppState) -> &'static str {
    let mut connection = match state.redis.get_multiplexed_async_connection().await {
        Ok(connection) => connection,
        Err(error) => {
            tracing::error!(%error, "redis health check failed");
            return "down";
        }
    };

    match redis::cmd("PING")
        .query_async::<String>(&mut connection)
        .await
    {
        Ok(_) => "up",
        Err(error) => {
            tracing::error!(%error, "redis health check failed");
            "down"
        }
    }
}
