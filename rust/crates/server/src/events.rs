use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::auth::middleware::AuthContext;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RecordOperationSignature {
    pub object_name_singular: String,
    #[serde(default)]
    pub variables: RecordOperationVariables,
    #[serde(default)]
    pub fields: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RecordOperationVariables {
    #[serde(default)]
    pub filter: Option<Value>,
    #[serde(default)]
    pub order_by: Option<Value>,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub cursor_filter: Option<Value>,
}

#[derive(Clone, Default)]
pub struct EventStreamRegistry {
    streams: Arc<Mutex<HashMap<(Uuid, String), EventStream>>>,
}

struct EventStream {
    auth_key: String,
    queries: HashMap<String, RecordOperationSignature>,
    sender: mpsc::UnboundedSender<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventWriter {
    pub user_id: Option<Uuid>,
    pub workspace_member_id: Option<Uuid>,
}

impl EventWriter {
    pub fn from_auth(auth: &AuthContext) -> Self {
        Self {
            user_id: auth.user_id,
            workspace_member_id: auth.workspace_member_id,
        }
    }
}

impl EventStreamRegistry {
    pub fn open(
        &self,
        workspace_id: Uuid,
        event_stream_id: String,
        auth: &AuthContext,
    ) -> Option<mpsc::UnboundedReceiver<Value>> {
        if event_stream_id.is_empty() {
            return None;
        }
        let mut streams = self.streams.lock().expect("event stream registry");
        let key = (workspace_id, event_stream_id);
        if streams.contains_key(&key) {
            return None;
        }
        let (sender, receiver) = mpsc::unbounded_channel();
        streams.insert(
            key,
            EventStream {
                auth_key: auth_key(auth),
                queries: HashMap::new(),
                sender,
            },
        );
        Some(receiver)
    }

    pub fn remove(&self, workspace_id: Uuid, event_stream_id: &str) {
        self.streams
            .lock()
            .expect("event stream registry")
            .remove(&(workspace_id, event_stream_id.to_string()));
    }

    pub fn add_query(
        &self,
        workspace_id: Uuid,
        event_stream_id: &str,
        auth: &AuthContext,
        query_id: String,
        operation_signature: RecordOperationSignature,
    ) -> bool {
        if query_id.is_empty() || operation_signature.object_name_singular.is_empty() {
            return false;
        }
        let mut streams = self.streams.lock().expect("event stream registry");
        let Some(stream) = streams.get_mut(&(workspace_id, event_stream_id.to_string())) else {
            return false;
        };
        if stream.auth_key != auth_key(auth) {
            return false;
        }
        stream.queries.insert(query_id, operation_signature);
        true
    }

    pub fn remove_query(
        &self,
        workspace_id: Uuid,
        event_stream_id: &str,
        auth: &AuthContext,
        query_id: &str,
    ) -> bool {
        let mut streams = self.streams.lock().expect("event stream registry");
        let Some(stream) = streams.get_mut(&(workspace_id, event_stream_id.to_string())) else {
            return false;
        };
        if stream.auth_key != auth_key(auth) {
            return false;
        }
        stream.queries.remove(query_id).is_some()
    }

    pub fn publish_record(
        &self,
        workspace_id: Uuid,
        action: &str,
        object_name_singular: &str,
        record_id: &str,
        properties: Value,
        writer: Option<&EventWriter>,
    ) {
        let streams = self.streams.lock().expect("event stream registry");
        for ((stream_workspace_id, event_stream_id), stream) in streams.iter() {
            if *stream_workspace_id != workspace_id {
                continue;
            }
            let query_ids = stream
                .queries
                .iter()
                .filter(|(_, signature)| {
                    signature.object_name_singular == object_name_singular
                        && signature
                            .variables
                            .filter
                            .as_ref()
                            .is_none_or(|filter| filter.is_null() || filter == &json!({}))
                })
                .map(|(query_id, _)| Value::String(query_id.clone()))
                .collect::<Vec<_>>();
            if query_ids.is_empty() {
                continue;
            }
            let payload = json!({
                "eventStreamId": event_stream_id,
                "objectRecordEventsWithQueryIds": [{
                    "objectRecordEvent": {
                        "action": action,
                        "objectNameSingular": object_name_singular,
                        "recordId": record_id,
                        "userId": writer.and_then(|writer| writer.user_id).map(|id| Value::String(id.to_string())).unwrap_or(Value::Null),
                        "workspaceMemberId": writer.and_then(|writer| writer.workspace_member_id).map(|id| Value::String(id.to_string())).unwrap_or(Value::Null),
                        "properties": properties,
                    },
                    "queryIds": query_ids,
                }],
                "metadataEvents": [],
                "queueJobEvents": [],
            });
            let _ = stream.sender.send(payload);
        }
    }
}

fn auth_key(auth: &AuthContext) -> String {
    auth.user_workspace_id
        .or(auth.api_key_id)
        .or(auth.user_id)
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt_service::TokenType;

    fn auth(user_id: Uuid) -> AuthContext {
        AuthContext {
            token_type: TokenType::Access,
            user_id: Some(user_id),
            workspace_id: Some(Uuid::new_v4()),
            user_workspace_id: None,
            workspace_member_id: None,
            api_key_id: None,
            agent_id: None,
            auth_provider: None,
            is_impersonating: false,
        }
    }

    #[test]
    fn record_events_only_reach_matching_workspace_queries() {
        let registry = EventStreamRegistry::default();
        let workspace_id = Uuid::new_v4();
        let other_workspace_id = Uuid::new_v4();
        let user = auth(Uuid::new_v4());
        let mut receiver = registry
            .open(workspace_id, String::from("stream-1"), &user)
            .expect("stream");
        let mut other_receiver = registry
            .open(other_workspace_id, String::from("stream-2"), &user)
            .expect("other stream");
        assert!(registry.add_query(
            workspace_id,
            "stream-1",
            &user,
            String::from("people-query"),
            RecordOperationSignature {
                object_name_singular: String::from("person"),
                variables: RecordOperationVariables::default(),
                fields: None,
            },
        ));
        assert!(registry.add_query(
            other_workspace_id,
            "stream-2",
            &user,
            String::from("people-query"),
            RecordOperationSignature {
                object_name_singular: String::from("person"),
                variables: RecordOperationVariables::default(),
                fields: None,
            },
        ));

        registry.publish_record(
            workspace_id,
            "updated",
            "person",
            "person-1",
            json!({"updatedFields": ["name"]}),
            Some(&EventWriter {
                user_id: Some(user.user_id.expect("test user")),
                workspace_member_id: None,
            }),
        );

        let payload = receiver.try_recv().expect("matching event");
        assert_eq!(payload["eventStreamId"], "stream-1");
        assert_eq!(
            payload["objectRecordEventsWithQueryIds"][0]["queryIds"],
            json!(["people-query"])
        );
        assert!(other_receiver.try_recv().is_err());
        assert!(!registry.add_query(
            workspace_id,
            "stream-1",
            &auth(Uuid::new_v4()),
            String::from("other-query"),
            RecordOperationSignature {
                object_name_singular: String::from("person"),
                variables: RecordOperationVariables::default(),
                fields: None,
            },
        ));
        assert!(registry.remove_query(workspace_id, "stream-1", &user, "people-query"));
        registry.publish_record(
            workspace_id,
            "updated",
            "person",
            "person-1",
            json!({}),
            None,
        );
        assert!(receiver.try_recv().is_err());
    }
}
