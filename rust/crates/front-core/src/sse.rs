use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseEvent {
    ObjectRecord {
        event: ObjectRecordEvent,
        query_ids: Vec<String>,
    },
    Metadata(MetadataEvent),
    QueueJob(QueueJobEvent),
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
    object_record_events_with_query_ids: Option<ObjectRecordEventsWithQueryIds>,
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

        if event_name == "next" && !data_lines.is_empty() {
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
    let mut events = Vec::new();

    if let Some(record) = subscription.object_record_events_with_query_ids {
        events.push(SseEvent::ObjectRecord {
            event: record.object_record_event,
            query_ids: record.query_ids,
        });
    }

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

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decoder_matches_the_upstream_object_record_event_frame() {
        let frame = "event: next\ndata: {\"data\":{\"onEventSubscription\":{\"objectRecordEventsWithQueryIds\":{\"objectRecordEvent\":{\"action\":\"updated\",\"objectNameSingular\":\"person\",\"recordId\":\"p1\",\"properties\":{\"updatedFields\":[\"name\"]}},\"queryIds\":[\"q1\",\"q2\"]}}}}\n\n";
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
    fn malformed_event_json_is_an_error() {
        assert!(matches!(
            decode_frames("event: next\ndata: {broken}\n\n"),
            Err(SseError::Json(_))
        ));
    }
}
