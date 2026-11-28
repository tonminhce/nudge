use std::sync::{Arc, OnceLock};

use redis::Client;
use sqlx::PgPool;
use uuid::Uuid;

use crate::webhook::dispatch::{
    deliver_webhook_requests, transform_event_batch_to_webhook_requests,
    transform_metadata_event_batch_to_webhook_requests, MetadataEventBatchInput,
    RecordEventBatchInput,
};
use crate::webhook::matching::{
    find_webhooks_matching_event_name, find_webhooks_matching_metadata_event_name,
    WebhookSubscription,
};
use crate::webhook::rate_limit::{
    admit_webhook_event_count, build_webhook_response_event_properties, slice_admitted_events,
    WEBHOOK_CALL_RATE_LIMITING_LIMIT, WEBHOOK_CALL_RATE_LIMITING_TTL_IN_MS,
};
use crate::webhook::sender::{
    default_webhook_client, WebhookDeliveryOutcome, WebhookDeliveryRequest,
};

pub const WEBHOOK_RESPONSE_EVENT: &str = "Webhook Response";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookDispatchError {
    InvalidWorkspaceId(String),
    Database(String),
}

impl std::fmt::Display for WebhookDispatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidWorkspaceId(value) => {
                write!(formatter, "INVALID_WORKSPACE_ID: {value} is not a workspace id")
            }
            Self::Database(message) => write!(formatter, "WEBHOOK_SUBSCRIPTION_LOAD_FAILED: {message}"),
        }
    }
}

impl std::error::Error for WebhookDispatchError {}

pub fn throttled_webhook_calls_message(
    workspace_id: &str,
    dropped_call_count: u64,
    refill_per_window: u64,
    window_ms: u64,
) -> String {
    format!(
        "Dropped {dropped_call_count} webhook calls for workspace {workspace_id}, which exceeded {refill_per_window} webhook calls per {}s",
        window_ms / 1_000
    )
}

pub async fn load_workspace_webhooks(
    pool: &PgPool,
    workspace_id: &str,
) -> Result<Vec<WebhookSubscription>, WebhookDispatchError> {
    let workspace_uuid = Uuid::parse_str(workspace_id)
        .map_err(|_| WebhookDispatchError::InvalidWorkspaceId(workspace_id.to_string()))?;

    let rows = sqlx::query_as::<_, (Uuid, String, Vec<String>, String)>(
        "SELECT id, \"targetUrl\", operations, secret FROM core.webhook \
         WHERE \"workspaceId\" = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(workspace_uuid)
    .fetch_all(pool)
    .await
    .map_err(|error| WebhookDispatchError::Database(error.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|(id, target_url, operations, secret)| WebhookSubscription {
            id: id.to_string(),
            target_url,
            operations,
            secret,
        })
        .collect())
}

pub fn record_requests_for_webhooks(
    batch: &RecordEventBatchInput,
    webhooks: &[WebhookSubscription],
) -> Vec<WebhookDeliveryRequest> {
    let matched: Vec<WebhookSubscription> = find_webhooks_matching_event_name(webhooks, &batch.name)
        .into_iter()
        .cloned()
        .collect();

    transform_event_batch_to_webhook_requests(batch, &matched)
}

pub fn metadata_requests_for_webhooks(
    batch: &MetadataEventBatchInput,
    webhooks: &[WebhookSubscription],
) -> Vec<WebhookDeliveryRequest> {
    let matched: Vec<WebhookSubscription> =
        find_webhooks_matching_metadata_event_name(webhooks, &batch.name)
            .into_iter()
            .cloned()
            .collect();

    transform_metadata_event_batch_to_webhook_requests(batch, &matched)
}

fn log_webhook_response(workspace_id: &str, outcome: &WebhookDeliveryOutcome) {
    let properties = build_webhook_response_event_properties(
        &outcome.url,
        &outcome.webhook_id,
        &outcome.event_name,
        outcome.success,
        outcome.status,
        outcome.event_log_error(),
    );

    if outcome.success {
        tracing::info!(
            workspace_id,
            event = WEBHOOK_RESPONSE_EVENT,
            properties = %properties,
            "webhook responded"
        );
    } else {
        tracing::warn!(
            workspace_id,
            event = WEBHOOK_RESPONSE_EVENT,
            properties = %properties,
            "webhook call failed"
        );
    }
}

pub struct WebhookDispatcher {
    pool: PgPool,
    redis: Client,
    http: Arc<reqwest::Client>,
}

impl WebhookDispatcher {
    pub fn new(pool: PgPool, redis: Client) -> Self {
        Self {
            pool,
            redis,
            http: Arc::new(default_webhook_client()),
        }
    }

    pub fn with_http_client(pool: PgPool, redis: Client, http: Arc<reqwest::Client>) -> Self {
        Self { pool, redis, http }
    }

    pub async fn dispatch_record_events(
        &self,
        batch: &RecordEventBatchInput,
    ) -> Vec<WebhookDeliveryOutcome> {
        match load_workspace_webhooks(&self.pool, &batch.workspace_id).await {
            Ok(webhooks) => {
                self.deliver(
                    &batch.workspace_id,
                    record_requests_for_webhooks(batch, &webhooks),
                )
                .await
            }
            Err(error) => {
                tracing::error!(
                    workspace_id = batch.workspace_id.as_str(),
                    %error,
                    "loading the webhook subscriptions failed, dropping the batch"
                );
                Vec::new()
            }
        }
    }

    pub async fn dispatch_metadata_events(
        &self,
        batch: &MetadataEventBatchInput,
    ) -> Vec<WebhookDeliveryOutcome> {
        match load_workspace_webhooks(&self.pool, &batch.workspace_id).await {
            Ok(webhooks) => {
                self.deliver(
                    &batch.workspace_id,
                    metadata_requests_for_webhooks(batch, &webhooks),
                )
                .await
            }
            Err(error) => {
                tracing::error!(
                    workspace_id = batch.workspace_id.as_str(),
                    %error,
                    "loading the webhook subscriptions failed, dropping the metadata batch"
                );
                Vec::new()
            }
        }
    }

    async fn deliver(
        &self,
        workspace_id: &str,
        requests: Vec<WebhookDeliveryRequest>,
    ) -> Vec<WebhookDeliveryOutcome> {
        if requests.is_empty() {
            return Vec::new();
        }

        let admitted = self.admit_requests(workspace_id, requests).await;
        let outcomes = deliver_webhook_requests(Arc::clone(&self.http), admitted).await;

        for outcome in &outcomes {
            log_webhook_response(workspace_id, outcome);
        }

        outcomes
    }

    async fn admit_requests(
        &self,
        workspace_id: &str,
        requests: Vec<WebhookDeliveryRequest>,
    ) -> Vec<WebhookDeliveryRequest> {
        if requests.is_empty() {
            return requests;
        }

        let requested = requests.len() as u64;
        let admitted_count = match self.consume_tokens(workspace_id, requested).await {
            Ok(admitted_count) => admitted_count,
            Err(error) => {
                tracing::error!(
                    workspace_id,
                    %error,
                    "Usage limit enforcement degraded: the webhook speed bucket is unavailable"
                );

                return requests;
            }
        };

        if admitted_count < requested {
            tracing::warn!(
                "{}",
                throttled_webhook_calls_message(
                    workspace_id,
                    requested - admitted_count,
                    WEBHOOK_CALL_RATE_LIMITING_LIMIT,
                    WEBHOOK_CALL_RATE_LIMITING_TTL_IN_MS,
                )
            );
        }

        slice_admitted_events(requests, admitted_count)
    }

    async fn consume_tokens(
        &self,
        workspace_id: &str,
        requested: u64,
    ) -> Result<u64, redis::RedisError> {
        let mut connection = self.redis.get_multiplexed_async_connection().await?;

        admit_webhook_event_count(&mut connection, workspace_id, requested).await
    }
}

static WEBHOOK_DISPATCHER: OnceLock<WebhookDispatcher> = OnceLock::new();

pub fn install(pool: PgPool, redis: Client) -> bool {
    WEBHOOK_DISPATCHER.set(WebhookDispatcher::new(pool, redis)).is_ok()
}

pub fn dispatcher() -> Option<&'static WebhookDispatcher> {
    WEBHOOK_DISPATCHER.get()
}

pub fn is_installed() -> bool {
    WEBHOOK_DISPATCHER.get().is_some()
}

pub fn spawn_record_event_batch(
    workspace_id: &str,
    object_metadata_id: &str,
    object_metadata_name_singular: &str,
    action: &str,
    events: Vec<serde_json::Value>,
) -> bool {
    spawn_record_event_delivery(RecordEventBatchInput {
        name: format!("{object_metadata_name_singular}.{action}"),
        workspace_id: workspace_id.to_string(),
        object_metadata_id: object_metadata_id.to_string(),
        object_metadata_name_singular: object_metadata_name_singular.to_string(),
        events,
    })
}

pub fn spawn_record_event_delivery(batch: RecordEventBatchInput) -> bool {
    let Some(dispatcher) = dispatcher() else {
        return false;
    };

    if tokio::runtime::Handle::try_current().is_err() {
        return false;
    }

    tokio::spawn(async move {
        dispatcher.dispatch_record_events(&batch).await;
    });

    true
}

pub fn spawn_metadata_event_delivery(batch: MetadataEventBatchInput) -> bool {
    let Some(dispatcher) = dispatcher() else {
        return false;
    };

    if tokio::runtime::Handle::try_current().is_err() {
        return false;
    }

    tokio::spawn(async move {
        dispatcher.dispatch_metadata_events(&batch).await;
    });

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn subscription(id: &str, operations: &[&str]) -> WebhookSubscription {
        WebhookSubscription {
            id: id.to_string(),
            target_url: format!("https://{id}.target.test/hooks"),
            operations: operations.iter().map(|operation| operation.to_string()).collect(),
            secret: format!("secret-{id}"),
        }
    }

    fn record_batch(events: Vec<serde_json::Value>) -> RecordEventBatchInput {
        RecordEventBatchInput {
            name: "person.updated".to_string(),
            workspace_id: "11111111-1111-1111-1111-111111111111".to_string(),
            object_metadata_id: "22222222-2222-2222-2222-222222222222".to_string(),
            object_metadata_name_singular: "person".to_string(),
            events,
        }
    }

    #[test]
    fn only_subscribed_operations_produce_requests() {
        let webhooks = vec![
            subscription("wildcard", &["*.*"]),
            subscription("other", &["company.created"]),
        ];
        let requests = record_requests_for_webhooks(
            &record_batch(vec![json!({ "properties": { "after": { "id": "1" } } })]),
            &webhooks,
        );

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].webhook_id, "wildcard");
        assert_eq!(requests[0].event_name, "person.updated");
        assert_eq!(requests[0].secret.as_deref(), Some("secret-wildcard"));
        assert!(!requests[0].payload_json.contains("secret-wildcard"));
        assert!(requests[0].payload_json.contains("\"objectMetadata\""));
    }

    #[test]
    fn one_request_is_built_per_matched_webhook_and_event() {
        let webhooks = vec![
            subscription("first", &["person.*"]),
            subscription("second", &["*.updated"]),
        ];
        let batch = record_batch(vec![
            json!({ "properties": { "after": { "id": "1" } } }),
            json!({ "properties": { "before": { "id": "2" }, "after": null } }),
        ]);
        let requests = record_requests_for_webhooks(&batch, &webhooks);

        assert_eq!(requests.len(), 4);
        assert_eq!(
            requests
                .iter()
                .map(|request| request.webhook_id.as_str())
                .collect::<Vec<&str>>()
                .join(","),
            "first,first,second,second"
        );
    }

    #[test]
    fn empty_event_batches_produce_no_requests() {
        let webhooks = vec![subscription("wildcard", &["*.*"])];

        assert!(record_requests_for_webhooks(&record_batch(Vec::new()), &webhooks).is_empty());

        let metadata_batch = MetadataEventBatchInput {
            name: "metadata.view.updated".to_string(),
            workspace_id: "11111111-1111-1111-1111-111111111111".to_string(),
            user_id: Some("33333333-3333-3333-3333-333333333333".to_string()),
            api_key_id: None,
            events: Vec::new(),
        };

        assert!(metadata_requests_for_webhooks(&metadata_batch, &webhooks).is_empty());
    }

    #[test]
    fn metadata_batches_match_the_metadata_operations() {
        let webhooks = vec![
            subscription("metadata-any", &["metadata.*.*"]),
            subscription("metadata-view", &["metadata.view.updated"]),
            subscription("record-only", &["*.*"]),
            subscription("unrelated", &["metadata.workflow.created"]),
        ];
        let batch = MetadataEventBatchInput {
            name: "metadata.view.updated".to_string(),
            workspace_id: "11111111-1111-1111-1111-111111111111".to_string(),
            user_id: None,
            api_key_id: Some("44444444-4444-4444-4444-444444444444".to_string()),
            events: vec![json!({ "type": "updated", "properties": { "after": {} } })],
        };
        let requests = metadata_requests_for_webhooks(&batch, &webhooks);
        let matched: Vec<&str> = requests
            .iter()
            .map(|request| request.webhook_id.as_str())
            .collect();

        assert_eq!(matched, vec!["metadata-any", "metadata-view", "record-only"]);
        assert!(requests[0]
            .payload_json
            .contains("\"apiKeyId\":\"44444444-4444-4444-4444-444444444444\""));
    }

    #[test]
    fn throttled_calls_reuse_the_upstream_warning() {
        assert_eq!(
            throttled_webhook_calls_message(
                "workspace-1",
                3,
                WEBHOOK_CALL_RATE_LIMITING_LIMIT,
                WEBHOOK_CALL_RATE_LIMITING_TTL_IN_MS,
            ),
            "Dropped 3 webhook calls for workspace workspace-1, which exceeded 60 webhook calls per 60s"
        );
    }

    #[tokio::test]
    async fn unknown_workspace_ids_are_rejected_before_touching_the_pool() {
        let pool = PgPool::connect_lazy("postgresql://nudge-test@127.0.0.1:55432/nudge_test")
            .expect("a lazy pool never connects");
        let error = load_workspace_webhooks(&pool, "not-a-uuid")
            .await
            .expect_err("the workspace id must be a uuid");

        assert_eq!(
            error,
            WebhookDispatchError::InvalidWorkspaceId("not-a-uuid".to_string())
        );
        assert_eq!(
            error.to_string(),
            "INVALID_WORKSPACE_ID: not-a-uuid is not a workspace id"
        );
    }

    #[tokio::test]
    async fn an_unavailable_bucket_admits_every_request() {
        let pool = PgPool::connect_lazy("postgresql://nudge-test@127.0.0.1:55432/nudge_test")
            .expect("a lazy pool never connects");
        let redis = Client::open("redis://127.0.0.1:56379/0").expect("the test url is parseable");
        let dispatcher = WebhookDispatcher::new(pool, redis);
        let requests = record_requests_for_webhooks(
            &record_batch(vec![json!({ "properties": { "after": { "id": "1" } } })]),
            &[subscription("wildcard", &["*.*"])],
        );
        let admitted = dispatcher.admit_requests("workspace-1", requests.clone()).await;

        assert_eq!(admitted, requests);
    }

    #[test]
    fn delivery_outcomes_carry_the_upstream_response_fields() {
        let outcome = WebhookDeliveryOutcome {
            url: "https://target.test/hooks".to_string(),
            webhook_id: "webhook-1".to_string(),
            event_name: "person.updated".to_string(),
            status: Some(500),
            success: false,
            error: None,
            ssrf_blocked: false,
        };
        let properties = build_webhook_response_event_properties(
            &outcome.url,
            &outcome.webhook_id,
            &outcome.event_name,
            outcome.success,
            outcome.status,
            outcome.event_log_error(),
        );

        assert_eq!(
            serde_json::to_string(&properties).expect("the properties serialize"),
            "{\"success\":false,\"url\":\"https://target.test/hooks\",\"webhookId\":\"webhook-1\",\"eventName\":\"person.updated\",\"status\":500}"
        );
    }

    #[test]
    fn a_transport_failure_reports_no_error_field_like_upstream() {
        let outcome = WebhookDeliveryOutcome {
            url: "https://target.test/hooks".to_string(),
            webhook_id: "webhook-1".to_string(),
            event_name: "person.updated".to_string(),
            status: None,
            success: false,
            error: Some("error sending request for url (https://target.test/hooks)".to_string()),
            ssrf_blocked: false,
        };
        let properties = build_webhook_response_event_properties(
            &outcome.url,
            &outcome.webhook_id,
            &outcome.event_name,
            outcome.success,
            outcome.status,
            outcome.event_log_error(),
        );

        assert_eq!(
            serde_json::to_string(&properties).expect("the properties serialize"),
            "{\"success\":false,\"url\":\"https://target.test/hooks\",\"webhookId\":\"webhook-1\",\"eventName\":\"person.updated\"}"
        );
    }

    #[test]
    fn an_ssrf_blocked_delivery_reports_the_upstream_error_sentence() {
        let outcome = WebhookDeliveryOutcome::build(
            &WebhookDeliveryRequest {
                target_url: "http://169.254.169.254/latest/meta-data".to_string(),
                webhook_id: "webhook-1".to_string(),
                event_name: "person.updated".to_string(),
                secret: None,
                payload_json: "{}".to_string(),
            },
            None,
            Some(
                "requested host resolves to an internal IP address which is not allowed"
                    .to_string(),
            ),
        );
        let properties = build_webhook_response_event_properties(
            &outcome.url,
            &outcome.webhook_id,
            &outcome.event_name,
            outcome.success,
            outcome.status,
            outcome.event_log_error(),
        );

        assert!(outcome.ssrf_blocked);
        assert!(!outcome.success);
        assert_eq!(
            serde_json::to_string(&properties).expect("the properties serialize"),
            "{\"success\":false,\"url\":\"http://169.254.169.254/latest/meta-data\",\"webhookId\":\"webhook-1\",\"eventName\":\"person.updated\",\"error\":\"Webhook URL resolves to a private/internal IP address\"}"
        );
    }
}
