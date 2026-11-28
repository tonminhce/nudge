#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use serde_json::json;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::auth::middleware::AuthContext;
use crate::query::filter::SnapshotFilterSchema;
use crate::query::runner::actors::ActorContext;

use super::clock::EventStreamClock;
use super::clock::SystemClock;
use super::constants::ttl_refresh_interval_ms;
use super::constants::APPLICATION_KEEPALIVE_INTERVAL_MS;
use super::constants::EVENT_STREAM_TTL_MS;
use super::constants::TIMELINE_ACTIVITY_OBJECT_NAME;
use super::gate::resolve_broadcast_events;
use super::gate::MetadataBroadcastContext;
use super::gate::MetadataBroadcastEvent;
use super::gate::PermissionFlagAcl;
use super::payload::empty_payload;
use super::payload::metadata_payload;
use super::payload::queue_job_payload;
use super::payload::record_event_payload;
use super::payload::QueueJobEvent;
use super::payload::RecordEvent;
use super::record::actor_ids;
use super::record::RecordOperationSignature;

#[derive(Debug, Clone)]
pub struct MetadataEventBatch<'a> {
    pub workspace_id: Uuid,
    pub metadata_name: &'a str,
    pub updated_collection_hash: Option<&'a str>,
    pub context: MetadataBroadcastContext<'a>,
    pub events: Vec<MetadataBroadcastEvent>,
}

impl<'a> MetadataEventBatch<'a> {
    pub fn new(
        workspace_id: Uuid,
        metadata_name: &'a str,
        events: Vec<MetadataBroadcastEvent>,
    ) -> Self {
        Self {
            workspace_id,
            metadata_name,
            updated_collection_hash: None,
            context: MetadataBroadcastContext::workspace_wide(),
            events,
        }
    }

    pub fn with_updated_collection_hash(mut self, hash: Option<&'a str>) -> Self {
        self.updated_collection_hash = hash;
        self
    }

    pub fn with_context(mut self, context: MetadataBroadcastContext<'a>) -> Self {
        self.context = context;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamPrincipal {
    pub user_workspace_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub application_id: Option<Uuid>,
}

impl StreamPrincipal {
    pub fn from_auth(auth: &AuthContext) -> Self {
        Self {
            user_workspace_id: auth.user_workspace_id,
            api_key_id: auth.api_key_id,
            user_id: auth.user_id,
            application_id: auth.agent_id,
        }
    }

    pub fn is_scoped_to_its_user(&self) -> bool {
        self.application_id.is_none() && self.api_key_id.is_none()
    }

    pub fn is_authorized_for(&self, incoming: &Self) -> bool {
        if let Some(user_workspace_id) = incoming.user_workspace_id {
            return self.user_workspace_id == Some(user_workspace_id)
                && self.application_id == incoming.application_id;
        }
        if let Some(api_key_id) = incoming.api_key_id {
            return self.api_key_id == Some(api_key_id);
        }
        match incoming.user_id {
            Some(user_id) => {
                self.user_id == Some(user_id) && self.application_id == incoming.application_id
            }
            None => false,
        }
    }
}

struct RegisteredStream {
    principal: StreamPrincipal,
    queries: HashMap<String, RecordOperationSignature>,
    sender: mpsc::UnboundedSender<Value>,
    expires_at: u64,
}

#[derive(Clone)]
pub struct EventStreamRegistry {
    streams: Arc<Mutex<HashMap<(Uuid, String), RegisteredStream>>>,
    clock: Arc<dyn EventStreamClock>,
}

impl Default for EventStreamRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStreamRegistry {
    pub fn new() -> Self {
        Self {
            streams: Arc::new(Mutex::new(HashMap::new())),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(clock: Arc<dyn EventStreamClock>) -> Self {
        Self {
            streams: Arc::new(Mutex::new(HashMap::new())),
            clock,
        }
    }

    pub fn ttl_refresh_is_due(now_ms: u64, last_refresh_at_ms: u64) -> bool {
        now_ms.saturating_sub(last_refresh_at_ms) > ttl_refresh_interval_ms()
    }

    fn locked(&self) -> MutexGuard<'_, HashMap<(Uuid, String), RegisteredStream>> {
        let mut streams = self
            .streams
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let now = self.clock.now_ms();
        streams.retain(|_, stream| stream.expires_at > now);
        streams
    }

    pub fn open(
        &self,
        workspace_id: Uuid,
        event_stream_id: String,
        auth: &AuthContext,
    ) -> Option<mpsc::UnboundedReceiver<Value>> {
        if event_stream_id.is_empty() {
            return None;
        }
        let principal = StreamPrincipal::from_auth(auth);
        let now = self.clock.now_ms();
        let mut streams = self.locked();
        let key = (workspace_id, event_stream_id);
        if let Some(existing) = streams.get(&key) {
            if !existing.principal.is_authorized_for(&principal) {
                return None;
            }
        }
        let (sender, receiver) = mpsc::unbounded_channel();
        streams.insert(
            key,
            RegisteredStream {
                principal,
                queries: HashMap::new(),
                sender,
                expires_at: now.saturating_add(EVENT_STREAM_TTL_MS),
            },
        );
        Some(receiver)
    }

    pub fn remove(&self, workspace_id: Uuid, event_stream_id: &str) {
        self.locked()
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
        if query_id.is_empty() || operation_signature.validate().is_err() {
            return false;
        }
        let now = self.clock.now_ms();
        let mut streams = self.locked();
        let Some(stream) = streams.get_mut(&(workspace_id, event_stream_id.to_string())) else {
            return false;
        };
        if !stream
            .principal
            .is_authorized_for(&StreamPrincipal::from_auth(auth))
        {
            return false;
        }
        stream.queries.insert(query_id, operation_signature);
        stream.expires_at = now.saturating_add(EVENT_STREAM_TTL_MS);
        true
    }

    pub fn remove_query(
        &self,
        workspace_id: Uuid,
        event_stream_id: &str,
        auth: &AuthContext,
        query_id: &str,
    ) -> bool {
        let now = self.clock.now_ms();
        let mut streams = self.locked();
        let Some(stream) = streams.get_mut(&(workspace_id, event_stream_id.to_string())) else {
            return false;
        };
        if !stream
            .principal
            .is_authorized_for(&StreamPrincipal::from_auth(auth))
        {
            return false;
        }
        let removed = stream.queries.remove(query_id).is_some();
        if removed {
            stream.expires_at = now.saturating_add(EVENT_STREAM_TTL_MS);
        }
        removed
    }

    pub fn keepalive_interval() -> std::time::Duration {
        std::time::Duration::from_millis(APPLICATION_KEEPALIVE_INTERVAL_MS)
    }

    pub fn active_stream_count(&self, workspace_id: Uuid) -> usize {
        self.locked()
            .keys()
            .filter(|(stream_workspace_id, _)| *stream_workspace_id == workspace_id)
            .count()
    }

    pub fn refresh_ttl(&self, workspace_id: Uuid, event_stream_id: &str) -> bool {
        let now = self.clock.now_ms();
        let mut streams = self.locked();
        let key = (workspace_id, event_stream_id.to_string());
        let Some(stream) = streams.get_mut(&key) else {
            return false;
        };
        stream.expires_at = now.saturating_add(EVENT_STREAM_TTL_MS);
        true
    }

    pub fn heartbeat(
        &self,
        workspace_id: Uuid,
        event_stream_id: &str,
        last_refresh_at_ms: &mut u64,
    ) -> Option<Value> {
        let now = self.clock.now_ms();
        if Self::ttl_refresh_is_due(now, *last_refresh_at_ms) {
            *last_refresh_at_ms = now;
            self.refresh_ttl(workspace_id, event_stream_id);
        }
        let streams = self.locked();
        let stream = streams.get(&(workspace_id, event_stream_id.to_string()))?;
        let payload = empty_payload(event_stream_id);
        let _ = stream.sender.send(payload.clone());
        Some(payload)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the upstream publishRecord call signature used by every mutation path"
    )]
    pub fn publish_record(
        &self,
        workspace_id: Uuid,
        object_metadata_id: Uuid,
        action: &str,
        object_name_singular: &str,
        record_id: &str,
        properties: Value,
        schema: &dyn SnapshotFilterSchema,
        actor: Option<&ActorContext>,
    ) {
        if crate::webhook::service::is_installed()
            && object_name_singular != TIMELINE_ACTIVITY_OBJECT_NAME
        {
            let (user_id, workspace_member_id) = actor_ids(actor);
            let event = json!({
                "userId": user_id,
                "workspaceMemberId": workspace_member_id,
                "properties": &properties,
            });
            crate::webhook::service::spawn_record_event_batch(
                &workspace_id.to_string(),
                &object_metadata_id.to_string(),
                object_name_singular,
                action,
                vec![event],
            );
        }
        let streams = self.locked();
        for ((stream_workspace_id, event_stream_id), stream) in streams.iter() {
            if *stream_workspace_id != workspace_id {
                continue;
            }
            let query_ids = stream
                .queries
                .iter()
                .filter(|(_, signature)| {
                    signature.matches_record(action, object_name_singular, &properties, schema)
                })
                .map(|(query_id, _)| query_id.clone())
                .collect::<Vec<_>>();
            if query_ids.is_empty() {
                continue;
            }
            let (user_id, workspace_member_id) = actor_ids(actor);
            let payload = record_event_payload(
                event_stream_id,
                RecordEvent {
                    action,
                    object_name_singular,
                    record_id,
                    properties: &properties,
                    query_ids,
                    user_id,
                    workspace_member_id,
                },
            );
            let _ = stream.sender.send(payload);
        }
    }

    pub fn publish_metadata(&self, batch: &MetadataEventBatch<'_>, acl: &PermissionFlagAcl) {
        if batch.events.is_empty() {
            return;
        }

        let resolved = resolve_broadcast_events(
            batch.metadata_name,
            &batch.events,
            &batch.context,
            batch.updated_collection_hash,
        );
        if resolved.is_empty() {
            return;
        }

        let streams = self.locked();
        for ((stream_workspace_id, event_stream_id), stream) in streams.iter() {
            if *stream_workspace_id != batch.workspace_id {
                continue;
            }
            let is_scoped_to_its_user = stream.principal.is_scoped_to_its_user();
            let user_workspace_id = stream.principal.user_workspace_id;
            let delivered = resolved
                .iter()
                .filter(|event| event.may_reach(is_scoped_to_its_user, user_workspace_id, acl))
                .collect::<Vec<_>>();
            if delivered.is_empty() {
                continue;
            }
            let payload = metadata_payload(event_stream_id, &delivered);
            let _ = stream.sender.send(payload);
        }
    }

    pub fn publish_queue_job(
        &self,
        workspace_id: Uuid,
        user_workspace_id: Option<Uuid>,
        event: &QueueJobEvent,
    ) {
        let streams = self.locked();
        for ((stream_workspace_id, event_stream_id), stream) in streams.iter() {
            if *stream_workspace_id != workspace_id {
                continue;
            }
            if user_workspace_id.is_some()
                && stream.principal.user_workspace_id != user_workspace_id
            {
                continue;
            }
            let payload = queue_job_payload(event_stream_id, event);
            let _ = stream.sender.send(payload);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt_service::TokenType;
    use crate::events::clock::ManualClock;
    use crate::events::constants::ttl_refresh_interval_ms;
    use crate::events::constants::WORKFLOWS_PERMISSION_FLAG;
    use crate::events::gate::MetadataEventType;
    use crate::events::payload::QueueJobState;
    use serde_json::json;
    use std::collections::HashSet;

    fn auth_with(principal: StreamPrincipal) -> AuthContext {
        AuthContext {
            token_type: TokenType::Access,
            user_id: principal.user_id,
            workspace_id: Some(Uuid::new_v4()),
            user_workspace_id: principal.user_workspace_id,
            workspace_member_id: None,
            api_key_id: principal.api_key_id,
            agent_id: principal.application_id,
            auth_provider: None,
            is_impersonating: false,
        }
    }

    fn principal(user_workspace_id: Option<Uuid>) -> StreamPrincipal {
        StreamPrincipal {
            user_workspace_id,
            api_key_id: None,
            user_id: Some(Uuid::new_v4()),
            application_id: None,
        }
    }

    fn workflow_event(record_id: &str) -> MetadataBroadcastEvent {
        MetadataBroadcastEvent::new(
            MetadataEventType::Updated,
            record_id,
            json!({
                "updatedFields": ["name"],
                "before": {"id": record_id, "name": "Old"},
                "after": {"id": record_id, "name": "New"}
            }),
        )
    }

    fn gated_batch(workspace_id: Uuid) -> MetadataEventBatch<'static> {
        MetadataEventBatch {
            workspace_id,
            metadata_name: "workflow",
            updated_collection_hash: None,
            context: MetadataBroadcastContext::workspace_wide(),
            events: vec![workflow_event("workflow-1")],
        }
    }

    fn queue_job(job_id: &str) -> QueueJobEvent {
        QueueJobEvent {
            job_id: String::from(job_id),
            state: QueueJobState::from_driver_state("completed").expect("state"),
            attempts_made: 1,
            failed_reason: None,
            progress: Some(100),
            enqueued_at: 1,
            started_at: None,
            finished_at: None,
        }
    }

    #[test]
    fn gated_workflow_events_never_reach_application_or_api_key_streams() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock);
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();
        let acl = {
            let mut allowed = HashSet::new();
            allowed.insert(user_workspace_id);
            PermissionFlagAcl::new([(WORKFLOWS_PERMISSION_FLAG.to_string(), allowed)])
        };

        let mut api_key_principal = principal(Some(user_workspace_id));
        api_key_principal.api_key_id = Some(Uuid::new_v4());
        let mut application_principal = principal(Some(user_workspace_id));
        application_principal.application_id = Some(Uuid::new_v4());

        let mut api_key_stream = registry
            .open(
                workspace_id,
                "api-key".into(),
                &auth_with(api_key_principal),
            )
            .expect("api key stream");
        let mut application_stream = registry
            .open(
                workspace_id,
                "application".into(),
                &auth_with(application_principal),
            )
            .expect("application stream");
        let mut user_stream = registry
            .open(
                workspace_id,
                "user".into(),
                &auth_with(principal(Some(user_workspace_id))),
            )
            .expect("user stream");

        registry.publish_metadata(&gated_batch(workspace_id), &acl);

        assert!(api_key_stream.try_recv().is_err());
        assert!(application_stream.try_recv().is_err());
        let payload = user_stream
            .try_recv()
            .expect("gated event reaches the user stream");
        assert_eq!(
            payload["metadataEvents"][0]["properties"],
            json!({"updatedFields": ["name"], "before": {"id": "workflow-1"}, "after": {"id": "workflow-1"}})
        );
    }

    #[test]
    fn gated_delivery_requires_the_permission_flag_for_the_stream_user_workspace() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock);
        let workspace_id = Uuid::new_v4();
        let member = Uuid::new_v4();
        let outsider = Uuid::new_v4();
        let mut member_stream = registry
            .open(
                workspace_id,
                "member".into(),
                &auth_with(principal(Some(member))),
            )
            .expect("member stream");
        let mut outsider_stream = registry
            .open(
                workspace_id,
                "outsider".into(),
                &auth_with(principal(Some(outsider))),
            )
            .expect("outsider stream");

        let acl = PermissionFlagAcl::default();
        registry.publish_metadata(&gated_batch(workspace_id), &acl);
        assert!(member_stream.try_recv().is_err());
        assert!(outsider_stream.try_recv().is_err());

        let mut allowed = HashSet::new();
        allowed.insert(member);
        let acl = PermissionFlagAcl::new([(WORKFLOWS_PERMISSION_FLAG.to_string(), allowed)]);
        registry.publish_metadata(&gated_batch(workspace_id), &acl);
        assert!(member_stream.try_recv().is_ok());
        assert!(outsider_stream.try_recv().is_err());
    }

    #[test]
    fn ungated_metadata_events_follow_only_the_recipient_list() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock);
        let workspace_id = Uuid::new_v4();
        let member = Uuid::new_v4();
        let mut stream = registry
            .open(
                workspace_id,
                "stream".into(),
                &auth_with(principal(Some(member))),
            )
            .expect("stream");
        let batch = MetadataEventBatch {
            workspace_id,
            metadata_name: "objectMetadata",
            updated_collection_hash: Some("hash-9"),
            context: MetadataBroadcastContext::workspace_wide(),
            events: vec![MetadataBroadcastEvent::new(
                MetadataEventType::Created,
                "object-1",
                json!({"after": {"labelSingular": "Task", "icon": "Icon"} }),
            )],
        };

        registry.publish_metadata(&batch, &PermissionFlagAcl::default());
        let payload = stream.try_recv().expect("workspace wide event");
        assert_eq!(
            payload["metadataEvents"][0]["metadataName"],
            "objectMetadata"
        );
        assert_eq!(payload["metadataEvents"][0]["type"], "created");
        assert_eq!(
            payload["metadataEvents"][0]["updatedCollectionHash"],
            "hash-9"
        );
        assert_eq!(
            payload["metadataEvents"][0]["properties"],
            json!({"after": {"labelSingular": "Task", "icon": "Icon"}})
        );
        assert_eq!(payload["objectRecordEventsWithQueryIds"], json!([]));
        assert_eq!(payload["queueJobEvents"], json!([]));

        let other_workspace_id = Uuid::new_v4();
        registry.publish_metadata(
            &MetadataEventBatch {
                workspace_id: other_workspace_id,
                ..batch
            },
            &PermissionFlagAcl::default(),
        );
        assert!(stream.try_recv().is_err());
    }

    #[test]
    fn private_workflow_recipient_lists_are_matched_per_stream() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock);
        let workspace_id = Uuid::new_v4();
        let creator = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mut creator_stream = registry
            .open(
                workspace_id,
                "creator".into(),
                &auth_with(principal(Some(creator))),
            )
            .expect("creator stream");
        let mut other_stream = registry
            .open(
                workspace_id,
                "other".into(),
                &auth_with(principal(Some(other))),
            )
            .expect("other stream");
        let batch = MetadataEventBatch {
            workspace_id,
            metadata_name: "workflow",
            updated_collection_hash: None,
            context: MetadataBroadcastContext::workspace_wide(),
            events: vec![MetadataBroadcastEvent::new(
                MetadataEventType::Updated,
                "workflow-1",
                json!({"after": {
                    "id": "workflow-1",
                    "visibility": "PRIVATE",
                    "createdByUserWorkspaceId": creator.to_string()
                }}),
            )],
        };
        let mut allowed = HashSet::new();
        allowed.insert(creator);
        allowed.insert(other);
        let acl = PermissionFlagAcl::new([(WORKFLOWS_PERMISSION_FLAG.to_string(), allowed)]);

        registry.publish_metadata(&batch, &acl);
        assert_eq!(
            creator_stream.try_recv().expect("creator event")["metadataEvents"][0]["recordId"],
            "workflow-1"
        );
        assert!(other_stream.try_recv().is_err());
    }

    #[test]
    fn queue_job_events_are_scoped_to_their_user_workspace() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock);
        let workspace_id = Uuid::new_v4();
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mut owner_stream = registry
            .open(
                workspace_id,
                "owner".into(),
                &auth_with(principal(Some(owner))),
            )
            .expect("owner stream");
        let mut other_stream = registry
            .open(
                workspace_id,
                "other".into(),
                &auth_with(principal(Some(other))),
            )
            .expect("other stream");

        registry.publish_queue_job(workspace_id, Some(owner), &queue_job("job-1"));
        let payload = owner_stream.try_recv().expect("scoped job");
        assert_eq!(payload["queueJobEvents"][0]["jobId"], "job-1");
        assert_eq!(payload["queueJobEvents"][0]["state"], "COMPLETED");
        assert_eq!(payload["metadataEvents"], json!([]));
        assert_eq!(payload["objectRecordEventsWithQueryIds"], json!([]));
        assert!(other_stream.try_recv().is_err());

        registry.publish_queue_job(workspace_id, None, &queue_job("job-2"));
        assert_eq!(
            owner_stream.try_recv().expect("broadcast job")["queueJobEvents"][0]["jobId"],
            "job-2"
        );
        assert_eq!(
            other_stream.try_recv().expect("scoped job")["queueJobEvents"][0]["jobId"],
            "job-2"
        );
    }

    #[test]
    fn queue_job_events_omit_unset_fields() {
        let event = QueueJobEvent {
            job_id: String::from("job-1"),
            state: QueueJobState::Failed,
            attempts_made: 2,
            failed_reason: Some(String::from("boom")),
            progress: None,
            enqueued_at: 10,
            started_at: Some(11),
            finished_at: None,
        };
        let payload = queue_job_payload("stream-1", &event);
        let job = &payload["queueJobEvents"][0];
        assert_eq!(
            job.as_object()
                .expect("object")
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                "jobId",
                "state",
                "attemptsMade",
                "failedReason",
                "enqueuedAt",
                "startedAt"
            ]
        );
        assert_eq!(job["failedReason"], "boom");
        assert!(job.get("progress").is_none());
        assert!(job.get("finishedAt").is_none());
    }

    #[test]
    fn every_chunk_carries_the_full_wire_shape() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock);
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();
        let auth = auth_with(principal(Some(user_workspace_id)));
        let mut stream = registry
            .open(workspace_id, "stream-1".into(), &auth)
            .expect("stream");

        let mut last_refresh = 0_u64;
        let heartbeat = registry
            .heartbeat(workspace_id, "stream-1", &mut last_refresh)
            .expect("heartbeat");
        assert_eq!(
            heartbeat
                .as_object()
                .expect("object")
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                "eventStreamId",
                "objectRecordEventsWithQueryIds",
                "metadataEvents",
                "queueJobEvents"
            ]
        );
        assert_eq!(heartbeat["eventStreamId"], "stream-1");
        assert_eq!(heartbeat["metadataEvents"], json!([]));
        assert_eq!(stream.try_recv().expect("heartbeat chunk"), heartbeat);

        registry.publish_queue_job(workspace_id, Some(user_workspace_id), &queue_job("job-1"));
        let payload = stream.try_recv().expect("queue job chunk");
        assert_eq!(payload.as_object().expect("object").len(), 4);
        assert_eq!(payload["eventStreamId"], "stream-1");
        assert_eq!(payload["objectRecordEventsWithQueryIds"], json!([]));
        assert_eq!(payload["metadataEvents"], json!([]));
        assert_eq!(
            EventStreamRegistry::keepalive_interval(),
            std::time::Duration::from_millis(APPLICATION_KEEPALIVE_INTERVAL_MS)
        );
    }

    #[test]
    fn streams_expire_once_the_injected_clock_passes_the_ttl() {
        let clock = Arc::new(ManualClock::new(1_000));
        let registry = EventStreamRegistry::with_clock(clock.clone());
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();
        let auth = auth_with(principal(Some(user_workspace_id)));
        let mut stream = registry
            .open(workspace_id, "stream-1".into(), &auth)
            .expect("stream");

        clock.advance(EVENT_STREAM_TTL_MS - 1);
        registry.publish_queue_job(workspace_id, None, &queue_job("job-1"));
        assert_eq!(
            stream.try_recv().expect("still alive")["queueJobEvents"][0]["jobId"],
            "job-1"
        );

        clock.advance(1);
        registry.publish_queue_job(workspace_id, None, &queue_job("job-2"));
        assert!(stream.try_recv().is_err());
        assert!(!registry.refresh_ttl(workspace_id, "stream-1"));
        assert!(registry
            .open(workspace_id, "stream-1".into(), &auth)
            .is_some());
    }

    #[test]
    fn activity_extends_the_ttl_and_query_registration_keeps_a_stream_alive() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock.clone());
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();
        let auth = auth_with(principal(Some(user_workspace_id)));
        let mut stream = registry
            .open(workspace_id, "stream-1".into(), &auth)
            .expect("stream");

        clock.advance(EVENT_STREAM_TTL_MS / 2);
        assert!(registry.refresh_ttl(workspace_id, "stream-1"));
        clock.advance(EVENT_STREAM_TTL_MS / 2);
        registry.publish_queue_job(workspace_id, None, &queue_job("job-1"));
        assert!(stream.try_recv().is_ok());

        clock.advance(EVENT_STREAM_TTL_MS / 4);
        assert!(registry.add_query(
            workspace_id,
            "stream-1",
            &auth,
            String::from("query-1"),
            RecordOperationSignature::parse(json!({
                "objectNameSingular": "person",
                "variables": {}
            }))
            .expect("signature"),
        ));
        assert!(registry.remove_query(workspace_id, "stream-1", &auth, "query-1"));
        clock.advance(EVENT_STREAM_TTL_MS - 1);
        registry.publish_queue_job(workspace_id, None, &queue_job("job-2"));
        assert_eq!(
            stream.try_recv().expect("kept alive")["eventStreamId"],
            "stream-1"
        );
    }

    #[test]
    fn heartbeat_chunks_are_throttled_to_a_fifth_of_the_ttl() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock.clone());
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();
        let mut stream = registry
            .open(
                workspace_id,
                "stream-1".into(),
                &auth_with(principal(Some(user_workspace_id))),
            )
            .expect("stream");
        let mut last_refresh = 0_u64;
        let refresh_window = ttl_refresh_interval_ms();

        clock.advance(refresh_window);
        registry
            .heartbeat(workspace_id, "stream-1", &mut last_refresh)
            .expect("heartbeat");
        assert_eq!(last_refresh, 0);
        assert_eq!(
            stream.try_recv().expect("chunk")["eventStreamId"],
            "stream-1"
        );

        clock.advance(1);
        registry
            .heartbeat(workspace_id, "stream-1", &mut last_refresh)
            .expect("heartbeat");
        assert_eq!(last_refresh, refresh_window + 1);
        assert_eq!(
            stream.try_recv().expect("chunk")["eventStreamId"],
            "stream-1"
        );
        assert!(stream.try_recv().is_err());
        assert_eq!(refresh_window, EVENT_STREAM_TTL_MS / 5);

        clock.advance(refresh_window + 1);
        assert_eq!(
            registry
                .heartbeat(workspace_id, "stream-1", &mut last_refresh)
                .expect("heartbeat")["queueJobEvents"],
            json!([])
        );
        assert_eq!(last_refresh, 2 * refresh_window + 2);
    }

    #[test]
    fn heartbeat_of_an_expired_stream_reports_absence() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock.clone());
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();
        let _stream = registry.open(
            workspace_id,
            "stream-1".into(),
            &auth_with(principal(Some(user_workspace_id))),
        );
        clock.advance(EVENT_STREAM_TTL_MS);
        let mut last_refresh = 0_u64;
        assert!(registry
            .heartbeat(workspace_id, "stream-1", &mut last_refresh)
            .is_none());
    }

    #[test]
    fn duplicate_stream_ids_are_replaced_for_the_same_principal() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock);
        let workspace_id = Uuid::new_v4();
        let owner = Uuid::new_v4();
        let auth = auth_with(principal(Some(owner)));
        let mut first = registry
            .open(workspace_id, "stream-1".into(), &auth)
            .expect("first stream");
        let mut reopened = registry
            .open(workspace_id, "stream-1".into(), &auth)
            .expect("reopened stream");

        registry.publish_queue_job(workspace_id, None, &queue_job("job-1"));
        assert!(first.try_recv().is_err());
        assert_eq!(
            reopened.try_recv().expect("newest stream")["queueJobEvents"][0]["jobId"],
            "job-1"
        );

        let stranger = principal(Some(Uuid::new_v4()));
        assert!(registry
            .open(workspace_id, "stream-1".into(), &auth_with(stranger))
            .is_none());
        registry.publish_queue_job(workspace_id, None, &queue_job("job-2"));
        assert_eq!(
            reopened.try_recv().expect("owner keeps the stream")["queueJobEvents"][0]["jobId"],
            "job-2"
        );
        assert!(registry.open(workspace_id, String::new(), &auth).is_none());
    }

    #[test]
    fn heartbeats_hold_a_stream_past_the_ttl_and_keep_delivery() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock.clone());
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();
        let auth = auth_with(principal(Some(user_workspace_id)));
        let mut stream = registry
            .open(workspace_id, "stream-1".into(), &auth)
            .expect("stream");
        let mut last_refresh = 0_u64;

        for _ in 0..6 {
            clock.advance(ttl_refresh_interval_ms() + 1);
            registry
                .heartbeat(workspace_id, "stream-1", &mut last_refresh)
                .expect("heartbeat");
        }
        assert!(clock.now_ms() > EVENT_STREAM_TTL_MS);
        assert_eq!(registry.active_stream_count(workspace_id), 1);

        registry.publish_queue_job(workspace_id, None, &queue_job("late-job"));
        let delivered = (0..10)
            .filter_map(|_| stream.try_recv().ok())
            .any(|payload| payload["queueJobEvents"][0]["jobId"] == "late-job");
        assert!(delivered);
    }

    #[test]
    fn removing_a_stream_stops_delivery() {
        let clock = Arc::new(ManualClock::new(0));
        let registry = EventStreamRegistry::with_clock(clock);
        let workspace_id = Uuid::new_v4();
        let user_workspace_id = Uuid::new_v4();
        let auth = auth_with(principal(Some(user_workspace_id)));
        let mut stream = registry
            .open(workspace_id, "stream-1".into(), &auth)
            .expect("stream");
        registry.remove(workspace_id, "stream-1");
        registry.publish_queue_job(workspace_id, None, &queue_job("job-1"));
        assert!(stream.try_recv().is_err());
        let mut last_refresh = 0_u64;
        assert!(registry
            .heartbeat(workspace_id, "stream-1", &mut last_refresh)
            .is_none());
    }
}
