#![allow(dead_code)]

use std::time::Duration;

use rand::Rng;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use uuid::Uuid;

use super::deferred;
use super::deferred::DeferredWorkspaceOperation;
use super::runner::RunnerError;
use crate::state::AppState;

pub const DEFERRED_POLL_INTERVAL: Duration = Duration::from_millis(5_000);
pub const DEFERRED_POLL_JITTER_MAX: Duration = Duration::from_millis(500);
pub const DEFERRED_CLAIM_BATCH: i64 = 10;
pub const STALE_RECOVERY_EVERY_TICKS: u64 = 120;

#[derive(Debug, Clone)]
pub struct DeferredWorkerShutdown(watch::Sender<bool>);

impl DeferredWorkerShutdown {
    pub fn shutdown(&self) {
        let _ = self.0.send(true);
    }
}

pub fn spawn_deferred_worker(state: AppState) -> JoinHandle<()> {
    let (sender, receiver) = watch::channel(false);

    tokio::spawn(async move {
        let _sender = sender;
        deferred_worker_loop(state, receiver).await;
    })
}

pub fn spawn_deferred_worker_with_shutdown(
    state: AppState,
    poll_interval: Duration,
) -> (DeferredWorkerShutdown, JoinHandle<()>) {
    let (sender, receiver) = watch::channel(false);
    let handle = tokio::spawn(async move {
        deferred_worker_loop_with_interval(state, receiver, poll_interval).await;
    });

    (DeferredWorkerShutdown(sender), handle)
}

pub async fn deferred_worker_loop(state: AppState, shutdown: watch::Receiver<bool>) {
    deferred_worker_loop_with_interval(state, shutdown, DEFERRED_POLL_INTERVAL).await;
}

async fn deferred_worker_loop_with_interval(
    state: AppState,
    mut shutdown: watch::Receiver<bool>,
    poll_interval: Duration,
) {
    let mut ticks: u64 = 0;

    if let Err(error) = deferred::reset_stale_in_progress(&state).await {
        tracing::warn!(%error, "deferred stale recovery pass failed at worker start");
    }

    loop {
        let wait = poll_interval + jitter();

        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }

        ticks += 1;

        if ticks % STALE_RECOVERY_EVERY_TICKS == 0 {
            if let Err(error) = deferred::reset_stale_in_progress(&state).await {
                tracing::warn!(%error, "deferred stale recovery pass failed");
            }
        }

        if let Err(error) = run_queue_pass(&state).await {
            tracing::warn!(%error, "deferred queue pass failed");
        }
    }
}

fn jitter() -> Duration {
    let micros = rand::thread_rng().gen_range(0..DEFERRED_POLL_JITTER_MAX.as_micros() as u64);

    Duration::from_micros(micros)
}

pub async fn run_queue_pass(state: &AppState) -> Result<usize, RunnerError> {
    let workspace_ids = pending_workspace_ids(state).await?;
    let mut executed = 0;

    for workspace_id in workspace_ids {
        match run_workspace_pass(state, workspace_id).await {
            Ok(count) => executed += count,
            Err(error) => {
                tracing::warn!(
                    %workspace_id,
                    %error,
                    "deferred workspace pass stopped on a failing operation"
                );
            }
        }
    }

    Ok(executed)
}

async fn pending_workspace_ids(state: &AppState) -> Result<Vec<Uuid>, RunnerError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT \"workspaceId\" FROM core.\"deferredWorkspaceOperation\" \
         WHERE status = 'PENDING' ORDER BY \"workspaceId\" ASC",
    )
    .fetch_all(&state.core_pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(workspace_id,)| workspace_id)
        .collect())
}

async fn run_workspace_pass(state: &AppState, workspace_id: Uuid) -> Result<usize, RunnerError> {
    let mut executed = 0;

    loop {
        let claimed =
            deferred::claim_pending_backoff(state, workspace_id, DEFERRED_CLAIM_BATCH).await?;

        if claimed.is_empty() {
            break;
        }

        for operation in claimed {
            if let Err(error) = deferred::execute_and_record(state, workspace_id, &operation).await
            {
                tracing::warn!(
                    operation_id = %operation.id,
                    operation_name = %operation.name,
                    %error,
                    "deferred operation failed and was rescheduled"
                );

                return Ok(executed);
            }

            executed += 1;
        }
    }

    Ok(executed)
}

pub fn pending_claimable_now(operation: &DeferredWorkspaceOperation, elapsed_ms: i64) -> bool {
    elapsed_ms >= deferred::next_attempt_backoff_ms(operation.attempts) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_constants_match_the_upstream_cadence_scale() {
        assert_eq!(DEFERRED_POLL_INTERVAL, Duration::from_millis(5_000));
        assert!(DEFERRED_POLL_JITTER_MAX < DEFERRED_POLL_INTERVAL);
        assert_eq!(DEFERRED_CLAIM_BATCH, 10);
        assert_eq!(STALE_RECOVERY_EVERY_TICKS * 5, 600);
    }

    #[test]
    fn backoff_gate_blocks_only_until_the_delay_elapses() {
        let operation = DeferredWorkspaceOperation {
            id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            application_universal_identifier: "app".to_string(),
            name: "build_index".to_string(),
            payload: serde_json::json!({ "indexMetadataId": Uuid::new_v4().to_string() }),
            position: 0,
            run_by_version: None,
            status: "PENDING".to_string(),
            attempts: 1,
            last_error: None,
        };

        assert!(!pending_claimable_now(&operation, 0));
        assert!(!pending_claimable_now(&operation, 29_999));
        assert!(pending_claimable_now(&operation, 30_000));
    }
}
