mod auth;
mod db;
mod metadata;
pub mod query;
mod routes;
mod standard_objects;
mod state;
mod workspace_migrations;

use anyhow::Context;
use nudge_core::Config;

use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env().context("reading the server configuration")?;
    init_tracing();

    let state = AppState::connect(&config)
        .await
        .context("connecting to postgres and redis")?;

    db::migrate::run(&state)
        .await
        .context("running the core schema migrations")?;

    tokio::spawn(crate::auth::session::cleanup_loop(state.clone()));
    crate::workspace_migrations::worker::spawn_deferred_worker(state.clone());

    let app = routes::router(state);
    let address = format!("0.0.0.0:{}", config.node_port);
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .with_context(|| format!("binding {address}"))?;

    tracing::info!(port = config.node_port, "nudge server listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("the http server stopped with an error")?;

    Ok(())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn shutdown_signal() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install the Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "failed to install the SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = interrupt => {}
        _ = terminate => {}
    }

    tracing::info!("shutdown signal received");
}
