use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use serde_json::json;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub status: &'static str,
    pub info: serde_json::Value,
    pub error: serde_json::Value,
    pub details: serde_json::Value,
}

pub async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<HealthReport>) {
    check_database(&state).await;
    check_redis(&state).await;

    (
        StatusCode::OK,
        Json(HealthReport {
            status: "ok",
            info: json!({}),
            error: json!({}),
            details: json!({}),
        }),
    )
}

async fn check_database(state: &AppState) {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.core_pool)
        .await
    {
        Ok(_) => {}
        Err(error) => {
            tracing::error!(%error, "database health check failed");
        }
    }
}

async fn check_redis(state: &AppState) {
    let connection = state.redis.get_multiplexed_async_connection().await;

    let mut connection = match connection {
        Ok(connection) => connection,
        Err(error) => {
            tracing::error!(%error, "redis health check failed");
            return;
        }
    };

    if let Err(error) = redis::cmd("PING")
        .query_async::<String>(&mut connection)
        .await
    {
        tracing::error!(%error, "redis health check failed");
    }
}
