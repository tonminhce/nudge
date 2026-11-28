use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::client::BrowserGraphQLClient;
use crate::client::ClientError;
use crate::client::Endpoint;
use crate::client::GraphQLClient;
use crate::client::RequestOptions;

pub const ON_EVENT_SUBSCRIPTION_QUERY: &str = r#"subscription OnEventSubscription($eventStreamId: String!) {
  onEventSubscription(eventStreamId: $eventStreamId) {
    eventStreamId
    objectRecordEventsWithQueryIds {
      objectRecordEvent {
        action
        objectNameSingular
        recordId
        userId
        workspaceMemberId
        properties { updatedFields before after diff }
      }
      queryIds
    }
    metadataEvents {
      type
      metadataName
      recordId
      updatedCollectionHash
      properties { updatedFields before after diff }
    }
    queueJobEvents { jobId state attemptsMade failedReason progress enqueuedAt startedAt finishedAt }
  }
}"#;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordOperationSignature {
    pub object_name_singular: String,
    pub variables: RecordOperationVariables,
    #[serde(default)]
    pub fields: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordOperationVariables {
    #[serde(default)]
    pub filter: Option<Value>,
    #[serde(default)]
    pub order_by: Option<Value>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub cursor_filter: Option<RecordCursorFilter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordCursorFilter {
    pub cursor: String,
    pub cursor_direction: RecordCursorDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordCursorDirection {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseEvent {
    ObjectRecord {
        event: ObjectRecordEvent,
        query_ids: Vec<String>,
    },
    Metadata(MetadataEvent),
    QueueJob(QueueJobEvent),
    Ready {
        event_stream_id: String,
    },
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRecordEvent {
    pub action: String,
    pub object_name_singular: String,
    pub record_id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub workspace_member_id: Option<String>,
    #[serde(default)]
    pub properties: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub metadata_name: String,
    pub record_id: String,
    #[serde(default)]
    pub updated_collection_hash: Option<String>,
    #[serde(default)]
    pub properties: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueJobEvent {
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub job_name: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub attempts_made: Option<i64>,
    #[serde(default)]
    pub failed_reason: Option<String>,
    #[serde(default)]
    pub progress: Option<Value>,
    #[serde(default)]
    pub enqueued_at: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
    #[serde(default)]
    pub properties: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ObjectRecordEventsWithQueryIds {
    object_record_event: ObjectRecordEvent,
    #[serde(default)]
    query_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnEventSubscription {
    #[serde(default)]
    event_stream_id: Option<String>,
    #[serde(default)]
    object_record_events_with_query_ids: Vec<ObjectRecordEventsWithQueryIds>,
    #[serde(default)]
    metadata_events: Vec<MetadataEvent>,
    #[serde(default)]
    queue_job_events: Vec<QueueJobEvent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventEnvelope {
    data: EventData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventData {
    on_event_subscription: OnEventSubscription,
}

#[derive(Debug, thiserror::Error)]
pub enum SseError {
    #[error("invalid SSE JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("SSE data is missing the onEventSubscription payload")]
    MissingPayload,
}

pub fn decode_frames(input: &str) -> Result<Vec<SseEvent>, SseError> {
    let mut events = Vec::new();

    for frame in input.split("\n\n") {
        let mut event_name = String::new();
        let mut data_lines = Vec::new();

        for line in frame.lines() {
            if let Some(name) = line.strip_prefix("event:") {
                event_name = name.trim().to_string();
            }
            if let Some(data) = line.strip_prefix("data:") {
                data_lines.push(data.strip_prefix(' ').unwrap_or(data));
            }
        }

        if (event_name.is_empty() || event_name == "next") && !data_lines.is_empty() {
            let data = data_lines.join("\n");
            events.extend(decode_event(&data)?);
        }
    }

    Ok(events)
}

fn decode_event(data: &str) -> Result<Vec<SseEvent>, SseError> {
    if data.trim() == "{}" {
        return Ok(vec![SseEvent::Ping]);
    }

    let envelope: EventEnvelope = serde_json::from_str(data)?;
    let subscription = envelope.data.on_event_subscription;
    let event_stream_id = subscription.event_stream_id.clone();
    let mut events = Vec::new();

    events.extend(
        subscription
            .object_record_events_with_query_ids
            .into_iter()
            .map(|record| SseEvent::ObjectRecord {
                event: record.object_record_event,
                query_ids: record.query_ids,
            }),
    );

    events.extend(
        subscription
            .metadata_events
            .into_iter()
            .map(SseEvent::Metadata),
    );
    events.extend(
        subscription
            .queue_job_events
            .into_iter()
            .map(SseEvent::QueueJob),
    );

    if events.is_empty() {
        if let Some(event_stream_id) = event_stream_id {
            events.push(SseEvent::Ready { event_stream_id });
        }
    }

    Ok(events)
}

pub async fn register_event_query(
    base_url: &str,
    token: &str,
    event_stream_id: &str,
    query_id: &str,
    operation_signature: &RecordOperationSignature,
) -> Result<bool, ClientError> {
    let query = r#"mutation AddQueryToEventStream($input: AddQuerySubscriptionInput!) {
      addQueryToEventStream(input: $input)
    }"#;
    let data = BrowserGraphQLClient::new()
        .execute_data(
            base_url,
            query,
            RequestOptions::new(Endpoint::Metadata)
                .operation_name("AddQueryToEventStream")
                .variables(serde_json::json!({
                    "input": {
                        "eventStreamId": event_stream_id,
                        "queryId": query_id,
                        "operationSignature": operation_signature,
                    }
                }))
                .bearer_token(token),
        )
        .await?;
    serde_json::from_value(data["addQueryToEventStream"].clone()).map_err(ClientError::Json)
}

pub async fn unregister_event_query(
    base_url: &str,
    token: &str,
    event_stream_id: &str,
    query_id: &str,
) -> Result<bool, ClientError> {
    let query = r#"mutation RemoveQueryFromEventStream($input: RemoveQueryFromEventStreamInput!) {
      removeQueryFromEventStream(input: $input)
    }"#;
    let data = BrowserGraphQLClient::new()
        .execute_data(
            base_url,
            query,
            RequestOptions::new(Endpoint::Metadata)
                .operation_name("RemoveQueryFromEventStream")
                .variables(serde_json::json!({
                    "input": {
                        "eventStreamId": event_stream_id,
                        "queryId": query_id,
                    }
                }))
                .bearer_token(token),
        )
        .await?;
    serde_json::from_value(data["removeQueryFromEventStream"].clone()).map_err(ClientError::Json)
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct EventStreamCancellation {
    handle: futures_util::future::AbortHandle,
    controller: web_sys::AbortController,
}

#[cfg(target_arch = "wasm32")]
impl EventStreamCancellation {
    pub fn cancel(&self) {
        self.handle.abort();
        self.controller.abort();
    }
}

#[cfg(target_arch = "wasm32")]
pub fn subscribe_event_stream<F>(
    base_url: &str,
    token: &str,
    event_stream_id: &str,
    on_event: F,
) -> Result<
    (
        EventStreamCancellation,
        impl std::future::Future<Output = Result<(), ClientError>>,
    ),
    ClientError,
>
where
    F: FnMut(SseEvent),
{
    use futures_util::future::AbortHandle;
    use futures_util::future::Abortable;
    use futures_util::StreamExt;

    let (handle, registration) = AbortHandle::new_pair();
    let controller = web_sys::AbortController::new()
        .map_err(|error| ClientError::Transport(format!("{error:?}")))?;
    let request_controller = controller.clone();
    let base_url = base_url.trim_end_matches('/').to_string();
    let token = token.to_string();
    let event_stream_id = event_stream_id.to_string();
    let future = async move {
        let subscription = async move {
            let url = format!("{base_url}/metadata");
            let response = gloo_net::http::Request::post(&url)
                .header("Authorization", &format!("Bearer {token}"))
                .header("Accept", "text/event-stream")
                .abort_signal(Some(&request_controller.signal()))
                .json(&serde_json::json!({
                    "query": ON_EVENT_SUBSCRIPTION_QUERY,
                    "variables": {"eventStreamId": event_stream_id},
                }))?
                .send()
                .await?;
            if response.status() != 200 {
                return Err(ClientError::Response {
                    status: response.status(),
                    message: String::from("event stream request failed"),
                });
            }
            let body = response.body().ok_or(ClientError::MissingData)?;
            let mut stream = wasm_streams::ReadableStream::from_raw(body).into_stream();
            let mut buffer = Vec::new();
            let mut on_event = on_event;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|error| ClientError::Transport(format!("{error:?}")))?;
                buffer.extend_from_slice(&js_sys::Uint8Array::from(chunk).to_vec());
                consume_event_frames(&mut buffer, &mut on_event)?;
            }
            if !buffer.is_empty() {
                consume_event_frames(&mut buffer, &mut on_event)?;
            }
            Ok(())
        };
        match Abortable::new(subscription, registration).await {
            Ok(result) => result,
            Err(_) => Ok(()),
        }
    };
    Ok((EventStreamCancellation { handle, controller }, future))
}

#[cfg(target_arch = "wasm32")]
fn consume_event_frames<F>(buffer: &mut Vec<u8>, on_event: &mut F) -> Result<(), ClientError>
where
    F: FnMut(SseEvent),
{
    while let Some(position) = buffer.windows(2).position(|window| window == b"\n\n") {
        let frame = buffer.drain(..=position + 1).collect::<Vec<_>>();
        let frame =
            String::from_utf8(frame).map_err(|error| ClientError::Transport(error.to_string()))?;
        for event in decode_frames(&frame)? {
            on_event(event);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decoder_matches_the_upstream_object_record_event_frame() {
        let frame = "event: next\ndata: {\"data\":{\"onEventSubscription\":{\"objectRecordEventsWithQueryIds\":[{\"objectRecordEvent\":{\"action\":\"updated\",\"objectNameSingular\":\"person\",\"recordId\":\"p1\",\"properties\":{\"updatedFields\":[\"name\"]}},\"queryIds\":[\"q1\",\"q2\"]}]}}}\n\n";
        let events = decode_frames(frame).unwrap();
        assert_eq!(
            events,
            vec![SseEvent::ObjectRecord {
                event: ObjectRecordEvent {
                    action: "updated".into(),
                    object_name_singular: "person".into(),
                    record_id: "p1".into(),
                    user_id: None,
                    workspace_member_id: None,
                    properties: Some(json!({"updatedFields": ["name"]})),
                },
                query_ids: vec!["q1".into(), "q2".into()],
            }]
        );
    }

    #[test]
    fn decoder_accepts_unnamed_server_frames_and_signals_stream_readiness() {
        let frame = "data: {\"data\":{\"onEventSubscription\":{\"eventStreamId\":\"stream-1\",\"objectRecordEventsWithQueryIds\":[],\"metadataEvents\":[],\"queueJobEvents\":[]}}}\n\n";
        assert_eq!(
            decode_frames(frame).unwrap(),
            vec![SseEvent::Ready {
                event_stream_id: String::from("stream-1")
            }]
        );
    }

    #[test]
    fn decoder_preserves_batched_object_record_events() {
        let frame = "event: next\ndata: {\"data\":{\"onEventSubscription\":{\"objectRecordEventsWithQueryIds\":[{\"objectRecordEvent\":{\"action\":\"updated\",\"objectNameSingular\":\"person\",\"recordId\":\"p1\"},\"queryIds\":[\"q1\"]},{\"objectRecordEvent\":{\"action\":\"created\",\"objectNameSingular\":\"person\",\"recordId\":\"p2\"},\"queryIds\":[\"q1\",\"q2\"]}]}}}\n\n";
        let events = decode_frames(frame).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            SseEvent::ObjectRecord { event, query_ids }
                if event.record_id == "p1" && query_ids == &vec!["q1".to_string()]
        ));
        assert!(matches!(
            &events[1],
            SseEvent::ObjectRecord { event, query_ids }
                if event.action == "created" && query_ids == &vec!["q1".to_string(), "q2".to_string()]
        ));
    }

    #[test]
    fn decoder_matches_the_upstream_metadata_hash_event() {
        let frame = "event: next\ndata: {\"data\":{\"onEventSubscription\":{\"metadataEvents\":[{\"type\":\"updated\",\"metadataName\":\"objectMetadata\",\"recordId\":\"m1\",\"updatedCollectionHash\":\"h2\"}]}}}\n\n";
        assert_eq!(
            decode_frames(frame).unwrap(),
            vec![SseEvent::Metadata(MetadataEvent {
                event_type: "updated".into(),
                metadata_name: "objectMetadata".into(),
                record_id: "m1".into(),
                updated_collection_hash: Some("h2".into()),
                properties: None,
            })]
        );
    }

    #[test]
    fn decoder_handles_multiple_frames_and_keepalive_comments() {
        let frame = ": ping\n\nevent: next\ndata: {}\n\nevent: next\ndata: {\"data\":{\"onEventSubscription\":{\"queueJobEvents\":[{\"jobId\":\"j1\",\"state\":\"completed\"}]}}}\n\n";
        let events = decode_frames(frame).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], SseEvent::Ping);
        assert!(matches!(events[1], SseEvent::QueueJob(_)));
    }

    #[test]
    fn operation_signature_serializes_as_a_typed_camel_case_object() {
        let signature = RecordOperationSignature {
            object_name_singular: String::from("person"),
            variables: RecordOperationVariables {
                filter: Some(json!({"name": {"firstName": {"eq": "Ada"}}})),
                order_by: Some(json!([{"name": {"lastName": "AscNullsFirst"}}])),
                limit: Some(60),
                cursor_filter: None,
            },
            fields: Some(serde_json::from_value(json!({"id": true})).expect("fields")),
        };
        assert_eq!(
            serde_json::to_value(signature).unwrap(),
            json!({
                "objectNameSingular": "person",
                "variables": {
                    "filter": {"name": {"firstName": {"eq": "Ada"}}},
                    "orderBy": [{"name": {"lastName": "AscNullsFirst"}}],
                    "limit": 60,
                    "cursorFilter": null
                },
                "fields": {"id": true}
            })
        );
    }

    #[test]
    fn malformed_event_json_is_an_error() {
        assert!(matches!(
            decode_frames("event: next\ndata: {broken}\n\n"),
            Err(SseError::Json(_))
        ));
    }
}
