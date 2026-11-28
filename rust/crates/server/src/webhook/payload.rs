use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Map, Value};

pub struct RecordWebhookEventInput {
    pub target_url: String,
    pub event_name: String,
    pub object_metadata_id: String,
    pub object_metadata_name_singular: String,
    pub workspace_id: String,
    pub webhook_id: String,
    pub event_date: DateTime<Utc>,
    pub user_id: Option<String>,
    pub workspace_member_id: Option<String>,
    pub record: Value,
    pub updated_fields: Option<Vec<String>>,
}

pub struct MetadataWebhookEventInput {
    pub target_url: String,
    pub event_name: String,
    pub workspace_id: String,
    pub webhook_id: String,
    pub event_date: DateTime<Utc>,
    pub user_id: Option<String>,
    pub api_key_id: Option<String>,
    pub event: Value,
}

pub fn event_date_to_json(date: &DateTime<Utc>) -> String {
    date.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn insert_optional(payload: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        payload.insert(key.to_string(), Value::String(value));
    }
}

pub fn record_event_payload(input: &RecordWebhookEventInput) -> Value {
    let mut object_metadata = Map::new();
    object_metadata.insert(
        "id".to_string(),
        Value::String(input.object_metadata_id.clone()),
    );
    object_metadata.insert(
        "nameSingular".to_string(),
        Value::String(input.object_metadata_name_singular.clone()),
    );

    let mut payload = Map::new();
    payload.insert(
        "targetUrl".to_string(),
        Value::String(input.target_url.clone()),
    );
    payload.insert(
        "eventName".to_string(),
        Value::String(input.event_name.clone()),
    );
    payload.insert("objectMetadata".to_string(), Value::Object(object_metadata));
    payload.insert(
        "workspaceId".to_string(),
        Value::String(input.workspace_id.clone()),
    );
    payload.insert(
        "webhookId".to_string(),
        Value::String(input.webhook_id.clone()),
    );
    payload.insert(
        "eventDate".to_string(),
        Value::String(event_date_to_json(&input.event_date)),
    );
    insert_optional(&mut payload, "userId", input.user_id.clone());
    insert_optional(
        &mut payload,
        "workspaceMemberId",
        input.workspace_member_id.clone(),
    );
    payload.insert("record".to_string(), input.record.clone());

    if let Some(updated_fields) = &input.updated_fields {
        payload.insert(
            "updatedFields".to_string(),
            Value::Array(
                updated_fields
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<Value>>(),
            ),
        );
    }

    Value::Object(payload)
}

pub fn metadata_event_payload(input: &MetadataWebhookEventInput) -> Value {
    let mut payload = Map::new();
    payload.insert(
        "targetUrl".to_string(),
        Value::String(input.target_url.clone()),
    );
    payload.insert(
        "eventName".to_string(),
        Value::String(input.event_name.clone()),
    );
    payload.insert(
        "workspaceId".to_string(),
        Value::String(input.workspace_id.clone()),
    );
    payload.insert(
        "webhookId".to_string(),
        Value::String(input.webhook_id.clone()),
    );
    payload.insert(
        "eventDate".to_string(),
        Value::String(event_date_to_json(&input.event_date)),
    );
    insert_optional(&mut payload, "userId", input.user_id.clone());
    insert_optional(&mut payload, "apiKeyId", input.api_key_id.clone());
    payload.insert("event".to_string(), input.event.clone());

    Value::Object(payload)
}

pub fn serialize_payload(payload: &Value) -> String {
    serde_json::to_string(payload).expect("webhook payloads are JSON serializable")
}

pub fn remove_secret_from_webhook_record(record: &Value, is_webhook_event: bool) -> Value {
    if !is_webhook_event {
        return record.clone();
    }

    match record {
        Value::Object(map) => {
            let mut sanitized = map.clone();
            sanitized.remove("secret");

            Value::Object(sanitized)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhook::signature::generate_signature;
    use chrono::TimeZone;

    fn fixed_date() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 2, 10, 0, 0).unwrap()
    }

    fn record_input() -> RecordWebhookEventInput {
        let record = serde_json::json!({
            "id": "8f14e45f-ea0c-4b1a-9c3d-2e5f7a9c1d3b",
            "name": { "first": "Ada", "last": "O'Brien-Smith" },
            "email": "ada@example.com",
            "jobTitle": "CEO & Founder <dev/null>",
            "linkedinLink": { "label": "", "href": "https://linkedin.com/in/ada" },
            "description": "line1\nline2\ttab backslash \\ quote \" grüße \u{1F600}",
            "isPersonal": true,
            "numEmployees": 42,
            "oppAmount": 1234567.891,
            "tags": ["vip", "éè"],
            "createdAt": "2026-07-01T08:30:00.000Z",
            "settings": { "visibility": "workspace" }
        });

        RecordWebhookEventInput {
            target_url: "https://example.com/hooks/twenty".to_string(),
            event_name: "person.created".to_string(),
            object_metadata_id: "b0e2f2a6-3f1c-4d5e-8f7a-1c2b3d4e5f60".to_string(),
            object_metadata_name_singular: "person".to_string(),
            workspace_id: "7c9e6679-7425-40de-944b-e07fc1f90ae7".to_string(),
            webhook_id: "3fa85f64-5717-4562-b3fc-2c963f66afa6".to_string(),
            event_date: fixed_date(),
            user_id: Some("0e4c4b1a-6f3a-4b5c-9d2e-1a2b3c4d5e6f".to_string()),
            workspace_member_id: None,
            record,
            updated_fields: Some(vec!["email".to_string(), "name".to_string()]),
        }
    }

    #[test]
    fn record_payload_serialization_matches_node_stringify() {
        let json = serialize_payload(&record_event_payload(&record_input()));

        assert_eq!(
            json,
            "{\"targetUrl\":\"https://example.com/hooks/twenty\",\"eventName\":\"person.created\",\"objectMetadata\":{\"id\":\"b0e2f2a6-3f1c-4d5e-8f7a-1c2b3d4e5f60\",\"nameSingular\":\"person\"},\"workspaceId\":\"7c9e6679-7425-40de-944b-e07fc1f90ae7\",\"webhookId\":\"3fa85f64-5717-4562-b3fc-2c963f66afa6\",\"eventDate\":\"2026-07-02T10:00:00.000Z\",\"userId\":\"0e4c4b1a-6f3a-4b5c-9d2e-1a2b3c4d5e6f\",\"record\":{\"id\":\"8f14e45f-ea0c-4b1a-9c3d-2e5f7a9c1d3b\",\"name\":{\"first\":\"Ada\",\"last\":\"O'Brien-Smith\"},\"email\":\"ada@example.com\",\"jobTitle\":\"CEO & Founder <dev/null>\",\"linkedinLink\":{\"label\":\"\",\"href\":\"https://linkedin.com/in/ada\"},\"description\":\"line1\\nline2\\ttab backslash \\\\ quote \\\" grüße 😀\",\"isPersonal\":true,\"numEmployees\":42,\"oppAmount\":1234567.891,\"tags\":[\"vip\",\"éè\"],\"createdAt\":\"2026-07-01T08:30:00.000Z\",\"settings\":{\"visibility\":\"workspace\"}},\"updatedFields\":[\"email\",\"name\"]}"
        );
    }

    #[test]
    fn record_signature_matches_upstream_generated_vector() {
        let payload = record_event_payload(&record_input());
        let json = serialize_payload(&payload);
        let secret = "5f4dcc3b5aa765d61d8327deb882cf99a0b4f8e1d3a7c5e9f1029384756abcdef0123456789abcdef0123456789abcdef";
        let timestamp = "1751450400000";

        assert_eq!(
            generate_signature(secret, timestamp, &json),
            "4c124648d8982db9b83d3eb5e9cc442a4370dbf2508040445805c98d5d93fd78"
        );
    }

    #[test]
    fn metadata_signature_matches_upstream_generated_vector() {
        let input = MetadataWebhookEventInput {
            target_url: "https://example.com/hooks/meta".to_string(),
            event_name: "viewIndex.updated".to_string(),
            workspace_id: "7c9e6679-7425-40de-944b-e07fc1f90ae7".to_string(),
            webhook_id: "3fa85f64-5717-4562-b3fc-2c963f66afa6".to_string(),
            event_date: fixed_date(),
            user_id: None,
            api_key_id: Some("d5a5f9a2-6b1c-4b8f-9c2d-3e4f5a6b7c8d".to_string()),
            event: serde_json::json!({
                "type": "updated",
                "id": "7e5a2a10-1111-4a2b-9c3d-4e5f6a7b8c9d",
                "name": "My view",
                "operations": [{ "viewId": "1", "operation": "update", "updatedFields": ["key"] }],
                "properties": { "before": { "key": "old" }, "after": { "key": "new" } }
            }),
        };
        let json = serialize_payload(&metadata_event_payload(&input));

        assert_eq!(
            json,
            "{\"targetUrl\":\"https://example.com/hooks/meta\",\"eventName\":\"viewIndex.updated\",\"workspaceId\":\"7c9e6679-7425-40de-944b-e07fc1f90ae7\",\"webhookId\":\"3fa85f64-5717-4562-b3fc-2c963f66afa6\",\"eventDate\":\"2026-07-02T10:00:00.000Z\",\"apiKeyId\":\"d5a5f9a2-6b1c-4b8f-9c2d-3e4f5a6b7c8d\",\"event\":{\"type\":\"updated\",\"id\":\"7e5a2a10-1111-4a2b-9c3d-4e5f6a7b8c9d\",\"name\":\"My view\",\"operations\":[{\"viewId\":\"1\",\"operation\":\"update\",\"updatedFields\":[\"key\"]}],\"properties\":{\"before\":{\"key\":\"old\"},\"after\":{\"key\":\"new\"}}}}"
        );

        let secret = "5f4dcc3b5aa765d61d8327deb882cf99a0b4f8e1d3a7c5e9f1029384756abcdef0123456789abcdef0123456789abcdef";

        assert_eq!(
            generate_signature(secret, "1751450400000", &json),
            "1451931b8b493ea06aa842d13d48f88e92821d2d3fbfddd5a24b2f91d00b155e"
        );
    }

    #[test]
    fn remove_secret_only_for_webhook_events() {
        let record = serde_json::json!({ "id": "1", "secret": "s", "targetUrl": "u" });

        let sanitized = remove_secret_from_webhook_record(&record, true);
        assert_eq!(sanitized.get("secret"), None);
        assert_eq!(sanitized.as_object().unwrap().len(), 2);

        let untouched = remove_secret_from_webhook_record(&record, false);
        assert_eq!(untouched, record);
    }

    #[test]
    fn undefined_fields_are_omitted() {
        let mut input = record_input();
        input.user_id = None;
        input.workspace_member_id = None;
        input.updated_fields = None;
        let json = serialize_payload(&record_event_payload(&input));

        assert!(!json.contains("userId"));
        assert!(!json.contains("workspaceMemberId"));
        assert!(!json.contains("updatedFields"));
    }
}
