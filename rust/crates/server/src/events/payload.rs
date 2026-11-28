use serde_json::json;
use serde_json::Map;
use serde_json::Value;

use super::gate::BroadcastEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueJobState {
    Completed,
    Failed,
    Active,
    Waiting,
    Delayed,
    Prioritized,
    WaitingChildren,
}

impl QueueJobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Active => "ACTIVE",
            Self::Waiting => "WAITING",
            Self::Delayed => "DELAYED",
            Self::Prioritized => "PRIORITIZED",
            Self::WaitingChildren => "WAITING_CHILDREN",
        }
    }

    pub fn from_driver_state(raw: &str) -> Option<Self> {
        match raw {
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "active" => Some(Self::Active),
            "waiting" => Some(Self::Waiting),
            "delayed" => Some(Self::Delayed),
            "prioritized" => Some(Self::Prioritized),
            "waiting-children" => Some(Self::WaitingChildren),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueueJobEvent {
    pub job_id: String,
    pub state: QueueJobState,
    pub attempts_made: i64,
    pub failed_reason: Option<String>,
    pub progress: Option<i64>,
    pub enqueued_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

impl QueueJobEvent {
    pub fn to_json(&self) -> Value {
        let mut event = Map::new();
        event.insert(String::from("jobId"), json!(self.job_id));
        event.insert(String::from("state"), json!(self.state.as_str()));
        event.insert(String::from("attemptsMade"), json!(self.attempts_made));
        if let Some(failed_reason) = &self.failed_reason {
            event.insert(String::from("failedReason"), json!(failed_reason));
        }
        if let Some(progress) = self.progress {
            event.insert(String::from("progress"), json!(progress));
        }
        event.insert(String::from("enqueuedAt"), json!(self.enqueued_at));
        if let Some(started_at) = self.started_at {
            event.insert(String::from("startedAt"), json!(started_at));
        }
        if let Some(finished_at) = self.finished_at {
            event.insert(String::from("finishedAt"), json!(finished_at));
        }
        Value::Object(event)
    }
}

pub fn queue_job_progress_percentage(progress: Option<f64>) -> Option<i64> {
    let progress = progress.filter(|progress| progress.is_finite())?;
    Some(progress.round().clamp(0.0, 100.0) as i64)
}

pub fn metadata_event_json(event: &BroadcastEvent) -> Value {
    let mut metadata_event = Map::new();
    metadata_event.insert(String::from("metadataName"), json!(event.entity_name));
    metadata_event.insert(String::from("type"), json!(event.event_type.as_str()));
    metadata_event.insert(String::from("recordId"), json!(event.record_id));
    metadata_event.insert(String::from("properties"), event.properties.clone());
    if let Some(updated_collection_hash) = &event.updated_collection_hash {
        metadata_event.insert(
            String::from("updatedCollectionHash"),
            json!(updated_collection_hash),
        );
    }
    Value::Object(metadata_event)
}

fn payload_with(
    event_stream_id: &str,
    object_record_events: Vec<Value>,
    metadata_events: Vec<Value>,
    queue_job_events: Vec<Value>,
) -> Value {
    let mut payload = Map::new();
    payload.insert(String::from("eventStreamId"), json!(event_stream_id));
    payload.insert(
        String::from("objectRecordEventsWithQueryIds"),
        Value::Array(object_record_events),
    );
    payload.insert(
        String::from("metadataEvents"),
        Value::Array(metadata_events),
    );
    payload.insert(
        String::from("queueJobEvents"),
        Value::Array(queue_job_events),
    );
    Value::Object(payload)
}

pub struct RecordEvent<'a> {
    pub action: &'a str,
    pub object_name_singular: &'a str,
    pub record_id: &'a str,
    pub properties: &'a Value,
    pub query_ids: Vec<String>,
    pub user_id: Value,
    pub workspace_member_id: Value,
}

pub fn record_event_payload(event_stream_id: &str, event: RecordEvent<'_>) -> Value {
    payload_with(
        event_stream_id,
        vec![json!({
            "objectRecordEvent": {
                "action": event.action,
                "objectNameSingular": event.object_name_singular,
                "recordId": event.record_id,
                "userId": event.user_id,
                "workspaceMemberId": event.workspace_member_id,
                "properties": event.properties,
            },
            "queryIds": event.query_ids,
        })],
        Vec::new(),
        Vec::new(),
    )
}

pub fn metadata_payload(event_stream_id: &str, events: &[&BroadcastEvent]) -> Value {
    payload_with(
        event_stream_id,
        Vec::new(),
        events
            .iter()
            .map(|event| metadata_event_json(event))
            .collect(),
        Vec::new(),
    )
}

pub fn queue_job_payload(event_stream_id: &str, event: &QueueJobEvent) -> Value {
    payload_with(
        event_stream_id,
        Vec::new(),
        Vec::new(),
        vec![event.to_json()],
    )
}

pub fn empty_payload(event_stream_id: &str) -> Value {
    payload_with(event_stream_id, Vec::new(), Vec::new(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_states_map_to_the_upstream_job_state_enum() {
        let cases = [
            ("completed", "COMPLETED"),
            ("failed", "FAILED"),
            ("active", "ACTIVE"),
            ("waiting", "WAITING"),
            ("delayed", "DELAYED"),
            ("prioritized", "PRIORITIZED"),
            ("waiting-children", "WAITING_CHILDREN"),
        ];

        for (driver_state, expected) in cases {
            assert_eq!(
                QueueJobState::from_driver_state(driver_state).map(QueueJobState::as_str),
                Some(expected)
            );
        }
        assert_eq!(QueueJobState::from_driver_state("moved"), None);
        assert_eq!(QueueJobState::from_driver_state(""), None);
    }

    #[test]
    fn progress_rounds_into_the_upstream_zero_to_hundred_percent_band() {
        assert_eq!(queue_job_progress_percentage(Some(12.4)), Some(12));
        assert_eq!(queue_job_progress_percentage(Some(12.6)), Some(13));
        assert_eq!(queue_job_progress_percentage(Some(0.5)), Some(1));
        assert_eq!(queue_job_progress_percentage(Some(-3.0)), Some(0));
        assert_eq!(queue_job_progress_percentage(Some(140.0)), Some(100));
        assert_eq!(queue_job_progress_percentage(Some(f64::NAN)), None);
        assert_eq!(queue_job_progress_percentage(Some(f64::INFINITY)), None);
        assert_eq!(queue_job_progress_percentage(None), None);
    }
}
