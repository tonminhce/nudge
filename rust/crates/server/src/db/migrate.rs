use sqlx::migrate::Migrator;
use twenty_core::Result;

use crate::state::AppState;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/core");

pub async fn run(state: &AppState) -> Result<()> {
    MIGRATOR.run(&state.core_pool).await?;
    Ok(())
}
