use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::task::JoinSet;

use crate::webhook::matching::WebhookSubscription;
use crate::webhook::payload::{
    metadata_event_payload, record_event_payload, remove_secret_from_webhook_record,
    serialize_payload, MetadataWebhookEventInput, RecordWebhookEventInput,
};
use crate::webhook::sender::{call_webhook, WebhookDeliveryOutcome, WebhookDeliveryRequest};

pub const WEBHOOK_JOBS_CHUNK_SIZE: usize = 20;

pub struct RecordEventBatchInput {
    pub name: String,
    pub workspace_id: String,
    pub object_metadata_id: String,
    pub object_metadata_name_singular: String,
    pub events: Vec<Value>,
}

pub struct MetadataEventBatchInput {
    pub name: String,
    pub workspace_id: String,
    pub user_id: Option<String>,
    pub api_key_id: Option<String>,
    pub events: Vec<Value>,
}

fn event_name_to_record(event: &Value) -> Value {
    let properties = event.get("properties");

    properties
        .and_then(|properties| properties.get("after"))
        .filter(|after| !after.is_null())
        .or_else(|| {
            properties
                .and_then(|properties| properties.get("before"))
                .filter(|before| !before.is_null())
        })
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()))
}

fn event_name_updated_fields(event: &Value) -> Option<Vec<String>> {
    event
        .get("properties")
        .and_then(|properties| properties.get("updatedFields"))
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(|field| field.as_str().map(str::to_string))
                .collect()
        })
}

pub fn transform_event_to_webhook_record(
    event_name: &str,
    event: &Value,
) -> (Value, Option<Vec<String>>) {
    let name_singular = event_name.split('.').next().unwrap_or(event_name);
    let record = event_name_to_record(event);
    let updated_fields = event_name_updated_fields(event);

    (
        remove_secret_from_webhook_record(&record, name_singular == "webhook"),
        updated_fields,
    )
}

pub fn transform_event_batch_to_webhook_requests(
    batch: &RecordEventBatchInput,
    webhooks: &[WebhookSubscription],
) -> Vec<WebhookDeliveryRequest> {
    let mut requests = Vec::new();

    for webhook in webhooks {
        let event_date: DateTime<Utc> = Utc::now();

        for event in &batch.events {
            let (record, updated_fields) = transform_event_to_webhook_record(&batch.name, event);
            let payload = record_event_payload(&RecordWebhookEventInput {
                target_url: webhook.target_url.clone(),
                event_name: batch.name.clone(),
                object_metadata_id: batch.object_metadata_id.clone(),
                object_metadata_name_singular: batch.object_metadata_name_singular.clone(),
                workspace_id: batch.workspace_id.clone(),
                webhook_id: webhook.id.clone(),
                event_date,
                user_id: event
                    .get("userId")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                workspace_member_id: event
                    .get("workspaceMemberId")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                record,
                updated_fields,
            });

            requests.push(WebhookDeliveryRequest {
                target_url: webhook.target_url.clone(),
                webhook_id: webhook.id.clone(),
                event_name: batch.name.clone(),
                secret: Some(webhook.secret.clone()),
                payload_json: serialize_payload(&payload),
            });
        }
    }

    requests
}

pub fn transform_metadata_event_batch_to_webhook_requests(
    batch: &MetadataEventBatchInput,
    webhooks: &[WebhookSubscription],
) -> Vec<WebhookDeliveryRequest> {
    let mut requests = Vec::new();

    for webhook in webhooks {
        let event_date: DateTime<Utc> = Utc::now();

        for event in &batch.events {
            let payload = metadata_event_payload(&MetadataWebhookEventInput {
                target_url: webhook.target_url.clone(),
                event_name: batch.name.clone(),
                workspace_id: batch.workspace_id.clone(),
                webhook_id: webhook.id.clone(),
                event_date,
                user_id: batch.user_id.clone(),
                api_key_id: batch.api_key_id.clone(),
                event: event.clone(),
            });

            requests.push(WebhookDeliveryRequest {
                target_url: webhook.target_url.clone(),
                webhook_id: webhook.id.clone(),
                event_name: batch.name.clone(),
                secret: Some(webhook.secret.clone()),
                payload_json: serialize_payload(&payload),
            });
        }
    }

    requests
}

pub async fn deliver_webhook_requests(
    client: std::sync::Arc<reqwest::Client>,
    requests: Vec<WebhookDeliveryRequest>,
) -> Vec<WebhookDeliveryOutcome> {
    let mut outcomes = Vec::with_capacity(requests.len());

    for chunk in requests.chunks(WEBHOOK_JOBS_CHUNK_SIZE) {
        let mut tasks = JoinSet::new();

        for request in chunk {
            let client = std::sync::Arc::clone(&client);
            let request = request.clone();

            tasks.spawn(async move { call_webhook(&client, &request).await });
        }

        while let Some(outcome) = tasks.join_next().await {
            if let Ok(outcome) = outcome {
                outcomes.push(outcome);
            }
        }
    }

    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::TcpListener;

    fn subscription() -> WebhookSubscription {
        WebhookSubscription {
            id: "3fa85f64-5717-4562-b3fc-2c963f66afa6".to_string(),
            target_url: "http://127.0.0.1/hook".to_string(),
            operations: vec!["person.created".to_string()],
            secret: "secret".to_string(),
        }
    }

    #[test]
    fn record_extraction_prefers_after_then_before() {
        let created = serde_json::json!({
            "userId": "u1",
            "properties": { "after": { "id": "1" }, "updatedFields": ["name"] }
        });
        let (record, updated) = transform_event_to_webhook_record("person.created", &created);
        assert_eq!(record, serde_json::json!({ "id": "1" }));
        assert_eq!(updated, Some(vec!["name".to_string()]));

        let deleted = serde_json::json!({
            "properties": { "before": { "id": "2" } }
        });
        let (record, updated) = transform_event_to_webhook_record("person.deleted", &deleted);
        assert_eq!(record, serde_json::json!({ "id": "2" }));
        assert_eq!(updated, None);

        let empty = serde_json::json!({ "properties": {} });
        let (record, _) = transform_event_to_webhook_record("person.updated", &empty);
        assert_eq!(record, serde_json::json!({}));
    }

    #[test]
    fn webhook_records_have_secret_removed() {
        let event = serde_json::json!({
            "properties": { "after": { "id": "1", "secret": "s", "targetUrl": "u" } }
        });
        let (record, _) = transform_event_to_webhook_record("webhook.updated", &event);

        assert_eq!(record, serde_json::json!({ "id": "1", "targetUrl": "u" }));
    }

    #[test]
    fn batch_transform_produces_one_request_per_webhook_and_event() {
        let batch = RecordEventBatchInput {
            name: "person.created".to_string(),
            workspace_id: "w1".to_string(),
            object_metadata_id: "o1".to_string(),
            object_metadata_name_singular: "person".to_string(),
            events: vec![
                serde_json::json!({ "userId": "u1", "properties": { "after": { "id": "1" } } }),
                serde_json::json!({ "properties": { "after": { "id": "2" } } }),
            ],
        };

        let requests =
            transform_event_batch_to_webhook_requests(&batch, &[subscription(), subscription()]);

        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| request.payload_json.contains("person.created")));
        assert!(requests.iter().all(|request| request.secret.is_some()));

        let without_user = serde_json::from_str::<Value>(&requests[0].payload_json).unwrap();
        assert_eq!(without_user["userId"], "u1");
        let anonymous = serde_json::from_str::<Value>(&requests[1].payload_json).unwrap();
        assert!(anonymous.get("userId").is_none());
    }

    #[tokio::test]
    async fn delivery_runs_chunks_of_twenty_against_local_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let received = Arc::new(Mutex::new(0usize));
        let counter = Arc::clone(&received);

        let server = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let counter = Arc::clone(&counter);

                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    while let Ok(read) = socket.read(&mut buffer).await {
                        if read == 0 {
                            break;
                        }

                        let chunk = &buffer[..read];
                        if chunk.windows(4).any(|window| window == b"\r\n\r\n") {
                            *counter.lock().unwrap() += 1;
                            let _ = socket
                                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                                .await;
                            break;
                        }
                    }
                });
            }
        });

        let requests = (0..25)
            .map(|index| WebhookDeliveryRequest {
                target_url: format!("http://{}/hook", addr),
                webhook_id: format!("webhook-{}", index),
                event_name: "person.created".to_string(),
                secret: Some("secret".to_string()),
                payload_json: format!("{{\"index\":{}}}", index),
            })
            .collect::<Vec<WebhookDeliveryRequest>>();

        let outcomes = deliver_webhook_requests(
            std::sync::Arc::new(crate::webhook::sender::default_webhook_client()),
            requests,
        )
        .await;

        assert_eq!(outcomes.len(), 25);
        assert!(outcomes.iter().all(|outcome| outcome.success));
        assert_eq!(*received.lock().unwrap(), 25);

        server.abort();
    }

    #[test]
    fn chunk_size_matches_upstream_job_chunk() {
        assert_eq!(WEBHOOK_JOBS_CHUNK_SIZE, 20);
    }
}
