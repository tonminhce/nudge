use nudge_core::Result;
use sqlx::migrate::Migrator;
use sqlx::{Connection, PgConnection};

use crate::state::AppState;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/core");

pub async fn run(state: &AppState) -> Result<()> {
    if std::env::var("SKIP_MIGRATIONS")
        .ok()
        .map(|value| value.trim().eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(false)
    {
        return Ok(());
    }

    let mut connection = PgConnection::connect(&state.config.pg_database_url).await?;

    sqlx::query("SET search_path TO public")
        .execute(&mut connection)
        .await?;

    MIGRATOR.run(&mut connection).await?;

    let _ = connection.close().await;

    Ok(())
}
