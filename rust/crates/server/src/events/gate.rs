use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use super::constants::broadcast_property_keys_for_gated_entity;
use super::constants::is_view_child_metadata_name;
use super::constants::required_permission_flag_for_entity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataEventType {
    Created,
    Updated,
    Deleted,
}

impl MetadataEventType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Deleted => "deleted",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "created" => Some(Self::Created),
            "updated" => Some(Self::Updated),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }
}

pub fn compute_metadata_event_name(metadata_name: &str, event_type: MetadataEventType) -> String {
    format!("metadata.{metadata_name}.{}", event_type.as_str())
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetadataBroadcastEvent {
    pub event_type: MetadataEventType,
    pub record_id: String,
    pub properties: Value,
}

impl MetadataBroadcastEvent {
    pub fn new(
        event_type: MetadataEventType,
        record_id: impl Into<String>,
        properties: Value,
    ) -> Self {
        Self {
            event_type,
            record_id: record_id.into(),
            properties,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetadataBroadcastContext<'a> {
    parent_views: Option<&'a HashMap<String, Value>>,
    parent_workflows: Option<&'a HashMap<String, Value>>,
    morph_relation_heads: Option<&'a HashMap<String, Value>>,
}

impl<'a> MetadataBroadcastContext<'a> {
    pub fn workspace_wide() -> Self {
        Self::default()
    }

    pub fn for_metadata_name(
        metadata_name: &str,
        parent_views: &'a HashMap<String, Value>,
        parent_workflows: &'a HashMap<String, Value>,
        morph_relation_heads: &'a HashMap<String, Value>,
    ) -> Self {
        Self {
            parent_views: is_view_child_metadata_name(metadata_name).then_some(parent_views),
            parent_workflows: (metadata_name == "workflowVersion").then_some(parent_workflows),
            morph_relation_heads: (metadata_name == "fieldMetadata")
                .then_some(morph_relation_heads),
        }
    }

    pub fn with_parent_views(mut self, parent_views: Option<&'a HashMap<String, Value>>) -> Self {
        self.parent_views = parent_views;
        self
    }

    pub fn with_parent_workflows(
        mut self,
        parent_workflows: Option<&'a HashMap<String, Value>>,
    ) -> Self {
        self.parent_workflows = parent_workflows;
        self
    }

    pub fn with_morph_relation_heads(
        mut self,
        morph_relation_heads: Option<&'a HashMap<String, Value>>,
    ) -> Self {
        self.morph_relation_heads = morph_relation_heads;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BroadcastEvent {
    pub event_type: MetadataEventType,
    pub entity_name: String,
    pub record_id: String,
    pub properties: Value,
    pub updated_collection_hash: Option<String>,
    pub recipient_user_workspace_ids: Option<Vec<Uuid>>,
    pub required_permission_flag: Option<&'static str>,
}

impl BroadcastEvent {
    pub fn may_reach(
        &self,
        is_scoped_to_its_user: bool,
        user_workspace_id: Option<Uuid>,
        acl: &PermissionFlagAcl,
    ) -> bool {
        if let Some(permission_flag) = self.required_permission_flag {
            let is_stream_allowed = is_scoped_to_its_user
                && user_workspace_id.is_some_and(|id| acl.allows(permission_flag, id));
            if !is_stream_allowed {
                return false;
            }
        }

        let Some(recipients) = self.recipient_user_workspace_ids.as_ref() else {
            return true;
        };

        user_workspace_id.is_some_and(|id| recipients.contains(&id))
    }
}

pub fn resolve_broadcast_events(
    metadata_name: &str,
    events: &[MetadataBroadcastEvent],
    context: &MetadataBroadcastContext<'_>,
    updated_collection_hash: Option<&str>,
) -> Vec<BroadcastEvent> {
    let events = match context.morph_relation_heads {
        Some(heads) if metadata_name == "fieldMetadata" => {
            filter_morph_relation_duplicate_field_events(events, heads)
        }
        _ => events.to_vec(),
    };

    let mut recipients_by_record_id: HashMap<&str, Vec<Uuid>> = HashMap::new();
    for event in &events {
        let record = broadcast_event_record(event.event_type, &event.properties);
        if let Some(recipients) =
            resolve_recipient_user_workspace_ids(metadata_name, record, context)
        {
            recipients_by_record_id.insert(event.record_id.as_str(), recipients);
        }
    }

    events
        .iter()
        .map(|event| BroadcastEvent {
            event_type: event.event_type,
            entity_name: String::from(metadata_name),
            record_id: event.record_id.clone(),
            properties: pick_broadcast_event_properties(metadata_name, &event.properties),
            updated_collection_hash: updated_collection_hash.map(str::to_string),
            recipient_user_workspace_ids: recipients_by_record_id
                .get(event.record_id.as_str())
                .cloned(),
            required_permission_flag: required_permission_flag_for_entity(metadata_name),
        })
        .collect()
}

fn broadcast_event_record(event_type: MetadataEventType, properties: &Value) -> Option<&Value> {
    match event_type {
        MetadataEventType::Deleted => properties.get("before"),
        MetadataEventType::Created | MetadataEventType::Updated => properties.get("after"),
    }
}

pub fn pick_broadcast_event_properties(entity_name: &str, properties: &Value) -> Value {
    let Some(property_keys) = broadcast_property_keys_for_gated_entity(entity_name) else {
        return properties.clone();
    };

    let mut picked = Map::new();
    if let Some(updated_fields) = properties.get("updatedFields") {
        picked.insert(String::from("updatedFields"), updated_fields.clone());
    }
    for key in ["before", "after"] {
        let Some(record) = properties.get(key).filter(|record| !record.is_null()) else {
            continue;
        };
        picked.insert(
            String::from(key),
            pick_record_properties(record, property_keys),
        );
    }

    Value::Object(picked)
}

fn pick_record_properties(record: &Value, property_keys: &[&str]) -> Value {
    let mut picked = Map::new();
    for key in property_keys {
        if let Some(value) = record.get(key) {
            picked.insert((*key).to_string(), value.clone());
        }
    }
    Value::Object(picked)
}

pub fn resolve_recipient_user_workspace_ids(
    metadata_name: &str,
    record: Option<&Value>,
    context: &MetadataBroadcastContext<'_>,
) -> Option<Vec<Uuid>> {
    if let Some(parent_views) = context.parent_views {
        return match non_empty_string(record, "viewId")
            .and_then(|view_id| parent_views.get(view_id))
        {
            Some(view) => view_recipient_user_workspace_ids(view),
            None => Some(Vec::new()),
        };
    }

    let record = record.filter(|record| !record.is_null());

    match metadata_name {
        "view" => match record {
            Some(record) => view_recipient_user_workspace_ids(record),
            None => Some(Vec::new()),
        },
        "navigationMenuItem" => record
            .and_then(|record| non_empty_string(Some(record), "userWorkspaceId"))
            .map(user_workspace_id_recipients),
        "workflow" => match record {
            Some(record) => workflow_recipient_user_workspace_ids(record),
            None => None,
        },
        "workflowVersion" => {
            let core_workflow_id = non_empty_string(record, "coreWorkflowId");
            let parent_workflows = context.parent_workflows;
            let (Some(core_workflow_id), Some(parent_workflows)) =
                (core_workflow_id, parent_workflows)
            else {
                return None;
            };
            match parent_workflows.get(core_workflow_id) {
                Some(parent) => workflow_recipient_user_workspace_ids(parent),
                None => Some(Vec::new()),
            }
        }
        _ => None,
    }
}

fn view_recipient_user_workspace_ids(view: &Value) -> Option<Vec<Uuid>> {
    let visibility = effective_view_visibility(view).and_then(Value::as_str);
    if visibility == Some("WORKSPACE") {
        return None;
    }
    if visibility == Some("UNLISTED") {
        if let Some(created_by) = non_empty_string(Some(view), "createdByUserWorkspaceId") {
            return Some(user_workspace_id_recipients(created_by));
        }
    }
    Some(Vec::new())
}

fn workflow_recipient_user_workspace_ids(workflow: &Value) -> Option<Vec<Uuid>> {
    if non_empty_string(Some(workflow), "visibility") != Some("PRIVATE") {
        return None;
    }
    match non_empty_string(Some(workflow), "createdByUserWorkspaceId") {
        Some(created_by) => Some(user_workspace_id_recipients(created_by)),
        None => Some(Vec::new()),
    }
}

fn user_workspace_id_recipients(user_workspace_id: &str) -> Vec<Uuid> {
    Uuid::parse_str(user_workspace_id)
        .ok()
        .into_iter()
        .collect()
}

fn non_empty_string<'a>(record: Option<&'a Value>, key: &str) -> Option<&'a str> {
    record?
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn effective_view_visibility(view: &Value) -> Option<&Value> {
    let overrides = view.get("overrides").and_then(Value::as_object);
    if let Some(overrides) = overrides {
        let owner = view
            .get("applicationUniversalIdentifier")
            .and_then(Value::as_str);
        if let Some(visibility) = authored_override_visibility(overrides, owner) {
            return Some(visibility);
        }
    }
    view.get("visibility")
}

fn authored_override_visibility<'a>(
    overrides: &'a Map<String, Value>,
    owner: Option<&str>,
) -> Option<&'a Value> {
    if let Some(visibility) = overrides.get("visibility") {
        return Some(visibility);
    }
    let non_owner = overrides
        .iter()
        .filter(|(key, _)| owner != Some(key.as_str()))
        .filter_map(|(_, entry)| entry.get("visibility"));
    let owned = overrides
        .iter()
        .filter(|(key, _)| owner == Some(key.as_str()))
        .filter_map(|(_, entry)| entry.get("visibility"));

    non_owner.chain(owned).next()
}

pub fn filter_morph_relation_duplicate_field_events(
    events: &[MetadataBroadcastEvent],
    morph_relation_heads: &HashMap<String, Value>,
) -> Vec<MetadataBroadcastEvent> {
    let events_without_targets = events
        .iter()
        .filter(|event| morph_relation_target(event).is_none())
        .cloned()
        .collect::<Vec<_>>();

    let record_ids = events_without_targets
        .iter()
        .map(|event| event.record_id.as_str())
        .collect::<HashSet<_>>();

    let mut heads: Vec<(String, Value)> = Vec::new();
    for event in events {
        let Some(morph_id) = morph_relation_target(event) else {
            continue;
        };
        let Some(head) = morph_relation_heads.get(morph_id) else {
            continue;
        };
        let Some(head_id) = head.get("id").and_then(Value::as_str) else {
            continue;
        };
        if record_ids.contains(head_id) {
            continue;
        }
        match heads.iter_mut().find(|(known, _)| known == head_id) {
            Some(entry) => entry.1 = head.clone(),
            None => heads.push((head_id.to_string(), head.clone())),
        }
    }

    let head_events = heads
        .into_iter()
        .map(|(head_id, head)| MetadataBroadcastEvent {
            event_type: MetadataEventType::Updated,
            record_id: head_id,
            properties: json!({
                "updatedFields": [],
                "diff": {},
                "before": head,
                "after": head,
            }),
        })
        .collect::<Vec<_>>();

    events_without_targets
        .into_iter()
        .chain(head_events)
        .collect()
}

fn morph_relation_target(event: &MetadataBroadcastEvent) -> Option<&str> {
    let record = match event.event_type {
        MetadataEventType::Deleted => event.properties.get("before"),
        MetadataEventType::Created | MetadataEventType::Updated => event.properties.get("after"),
    }?;
    if record.get("type").and_then(Value::as_str) != Some("MORPH_RELATION") {
        return None;
    }
    let morph_id = record.get("morphId").and_then(Value::as_str)?;
    if morph_id.is_empty() {
        return None;
    }
    let is_head = record
        .get("universalIdentifier")
        .and_then(Value::as_str)
        .is_some_and(|universal_identifier| universal_identifier == morph_id);
    (!is_head).then_some(morph_id)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionFlagAcl {
    allowed_user_workspace_ids: HashMap<String, HashSet<Uuid>>,
}

impl PermissionFlagAcl {
    pub fn new(entries: impl IntoIterator<Item = (String, HashSet<Uuid>)>) -> Self {
        Self {
            allowed_user_workspace_ids: entries.into_iter().collect(),
        }
    }

    pub fn allows(&self, permission_flag: &str, user_workspace_id: Uuid) -> bool {
        self.allowed_user_workspace_ids
            .get(permission_flag)
            .is_some_and(|ids| ids.contains(&user_workspace_id))
    }

    pub fn is_empty(&self) -> bool {
        self.allowed_user_workspace_ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        event_type: MetadataEventType,
        record_id: &str,
        properties: Value,
    ) -> MetadataBroadcastEvent {
        MetadataBroadcastEvent::new(event_type, record_id, properties)
    }

    fn broadcast_event(
        recipients: Option<Vec<Uuid>>,
        required_permission_flag: Option<&'static str>,
    ) -> BroadcastEvent {
        BroadcastEvent {
            event_type: MetadataEventType::Updated,
            entity_name: String::from("workflow"),
            record_id: String::from("workflow-1"),
            properties: json!({"id": "workflow-1"}),
            updated_collection_hash: None,
            recipient_user_workspace_ids: recipients,
            required_permission_flag,
        }
    }

    fn acl_with(flag: &str, user_workspace_id: Uuid) -> PermissionFlagAcl {
        PermissionFlagAcl::new([(flag.to_string(), [user_workspace_id].into_iter().collect())])
    }

    #[test]
    fn gated_workflow_properties_keep_only_the_broadcast_keys() {
        let properties = json!({
            "updatedFields": ["name", "status"],
            "before": {"id": "workflow-1", "name": "Old", "status": "ACTIVE"},
            "after": {"id": "workflow-1", "name": "New", "status": "INACTIVE"},
            "diff": {"name": {"before": "Old", "after": "New"}}
        });
        assert_eq!(
            pick_broadcast_event_properties("workflow", &properties),
            json!({
                "updatedFields": ["name", "status"],
                "before": {"id": "workflow-1"},
                "after": {"id": "workflow-1"}
            })
        );
        assert_eq!(
            pick_broadcast_event_properties(
                "workflowVersion",
                &json!({
                    "before": null,
                    "after": {"id": "version-1", "coreWorkflowId": "workflow-1", "name": "v1"}
                })
            ),
            json!({"after": {"id": "version-1", "coreWorkflowId": "workflow-1"}})
        );
        let untouched =
            json!({"before": {"labelSingular": "Task"}, "after": {"labelSingular": "Todo"}});
        assert_eq!(
            pick_broadcast_event_properties("objectMetadata", &untouched),
            untouched
        );
    }

    #[test]
    fn private_workflow_events_reach_only_the_creator() {
        let creator = Uuid::new_v4();
        let context = MetadataBroadcastContext::workspace_wide();
        let events = vec![event(
            MetadataEventType::Updated,
            "workflow-1",
            json!({
                "after": {
                    "id": "workflow-1",
                    "visibility": "PRIVATE",
                    "createdByUserWorkspaceId": creator.to_string()
                }
            }),
        )];
        let resolved = resolve_broadcast_events("workflow", &events, &context, None);
        assert_eq!(
            resolved[0].recipient_user_workspace_ids,
            Some(vec![creator])
        );
        assert_eq!(resolved[0].required_permission_flag, Some("WORKFLOWS"));
        assert_eq!(
            resolved[0].properties,
            json!({"after": {"id": "workflow-1"}})
        );

        let shared = vec![event(
            MetadataEventType::Updated,
            "workflow-2",
            json!({"after": {"id": "workflow-2", "visibility": "WORKSPACE"}}),
        )];
        assert_eq!(
            resolve_broadcast_events("workflow", &shared, &context, None)[0]
                .recipient_user_workspace_ids,
            None
        );

        let ownerless = vec![event(
            MetadataEventType::Created,
            "workflow-3",
            json!({"after": {"id": "workflow-3", "visibility": "PRIVATE"}}),
        )];
        assert_eq!(
            resolve_broadcast_events("workflow", &ownerless, &context, None)[0]
                .recipient_user_workspace_ids,
            Some(Vec::new())
        );
    }

    #[test]
    fn deleted_records_resolve_recipients_from_the_before_image() {
        let creator = Uuid::new_v4();
        let events = vec![event(
            MetadataEventType::Deleted,
            "workflow-1",
            json!({
                "before": {"id": "workflow-1", "visibility": "PRIVATE", "createdByUserWorkspaceId": creator.to_string()},
                "after": null
            }),
        )];
        let resolved = resolve_broadcast_events(
            "workflow",
            &events,
            &MetadataBroadcastContext::workspace_wide(),
            None,
        );
        assert_eq!(
            resolved[0].recipient_user_workspace_ids,
            Some(vec![creator])
        );
    }

    #[test]
    fn workflow_versions_resolve_through_their_parent_and_fail_closed_when_it_is_gone() {
        let parent_id = String::from("workflow-1");
        let missing_parent = String::from("workflow-404");
        let creator = Uuid::new_v4();
        let parents = HashMap::from([(
            parent_id.clone(),
            json!({"id": parent_id, "visibility": "PRIVATE", "createdByUserWorkspaceId": creator.to_string()}),
        )]);
        let context =
            MetadataBroadcastContext::workspace_wide().with_parent_workflows(Some(&parents));
        let events = vec![
            event(
                MetadataEventType::Created,
                "version-1",
                json!({"after": {"id": "version-1", "coreWorkflowId": parent_id}}),
            ),
            event(
                MetadataEventType::Created,
                "version-2",
                json!({"after": {"id": "version-2", "coreWorkflowId": missing_parent}}),
            ),
            event(
                MetadataEventType::Created,
                "version-3",
                json!({"after": {"id": "version-3"}}),
            ),
        ];
        let resolved = resolve_broadcast_events("workflowVersion", &events, &context, None);
        assert_eq!(
            resolved[0].recipient_user_workspace_ids,
            Some(vec![creator])
        );
        assert_eq!(resolved[1].recipient_user_workspace_ids, Some(Vec::new()));
        assert_eq!(resolved[2].recipient_user_workspace_ids, None);
    }

    #[test]
    fn view_children_resolve_the_workspace_effective_visibility() {
        let creator = Uuid::new_v4();
        let parents = HashMap::from([
            (
                String::from("view-workspace"),
                json!({"id": "view-workspace", "visibility": "WORKSPACE"}),
            ),
            (
                String::from("view-unlisted"),
                json!({
                    "id": "view-unlisted",
                    "visibility": "UNLISTED",
                    "createdByUserWorkspaceId": creator.to_string()
                }),
            ),
            (
                String::from("view-ownerless"),
                json!({"id": "view-ownerless", "visibility": "UNLISTED"}),
            ),
            (
                String::from("view-overridden"),
                json!({
                    "id": "view-overridden",
                    "visibility": "WORKSPACE",
                    "createdByUserWorkspaceId": creator.to_string(),
                    "applicationUniversalIdentifier": "app-1",
                    "overrides": {
                        "app-1": {"visibility": "PRIVATE"},
                        "app-2": {"visibility": "UNLISTED"}
                    }
                }),
            ),
        ]);
        let context = MetadataBroadcastContext::workspace_wide().with_parent_views(Some(&parents));

        let resolve = |view_id: &str| {
            let events = vec![event(
                MetadataEventType::Updated,
                "field-1",
                json!({"after": {"viewId": view_id}}),
            )];
            resolve_broadcast_events("viewField", &events, &context, None)[0]
                .recipient_user_workspace_ids
                .clone()
        };

        assert_eq!(resolve("view-workspace"), None);
        assert_eq!(resolve("view-unlisted"), Some(vec![creator]));
        assert_eq!(resolve("view-ownerless"), Some(Vec::new()));
        assert_eq!(resolve("view-overridden"), Some(vec![creator]));
        assert_eq!(resolve("view-missing"), Some(Vec::new()));
    }

    #[test]
    fn authored_overrides_take_the_first_non_owner_entry_then_the_owner() {
        let creator = Uuid::new_v4();
        let parents = HashMap::from([(
            String::from("view-flat"),
            json!({
                "id": "view-flat",
                "visibility": "PRIVATE",
                "createdByUserWorkspaceId": creator.to_string(),
                "overrides": {"visibility": "WORKSPACE"}
            }),
        )]);
        let context = MetadataBroadcastContext::workspace_wide().with_parent_views(Some(&parents));
        let events = vec![event(
            MetadataEventType::Updated,
            "field-1",
            json!({"after": {"viewId": "view-flat"}}),
        )];
        assert_eq!(
            resolve_broadcast_events("viewField", &events, &context, None)[0]
                .recipient_user_workspace_ids,
            None
        );
    }

    #[test]
    fn navigation_menu_items_are_scoped_to_their_own_user_workspace() {
        let owner = Uuid::new_v4();
        let context = MetadataBroadcastContext::workspace_wide();
        let owned = vec![event(
            MetadataEventType::Created,
            "item-1",
            json!({"after": {"id": "item-1", "userWorkspaceId": owner.to_string()}}),
        )];
        assert_eq!(
            resolve_broadcast_events("navigationMenuItem", &owned, &context, None)[0]
                .recipient_user_workspace_ids,
            Some(vec![owner])
        );
        let shared = vec![event(
            MetadataEventType::Created,
            "item-2",
            json!({"after": {"id": "item-2"}}),
        )];
        assert_eq!(
            resolve_broadcast_events("navigationMenuItem", &shared, &context, None)[0]
                .recipient_user_workspace_ids,
            None
        );
    }

    #[test]
    fn gated_events_require_stream_scoping_and_the_permission_flag() {
        let user_workspace_id = Uuid::new_v4();
        let acl = acl_with("WORKFLOWS", user_workspace_id);
        let gated = broadcast_event(None, Some("WORKFLOWS"));

        assert!(gated.may_reach(true, Some(user_workspace_id), &acl));
        assert!(!gated.may_reach(false, Some(user_workspace_id), &acl));
        assert!(!gated.may_reach(true, Some(Uuid::new_v4()), &acl));
        assert!(!gated.may_reach(true, None, &acl));

        let unflagged = broadcast_event(None, None);
        assert!(unflagged.may_reach(false, None, &PermissionFlagAcl::default()));
    }

    #[test]
    fn recipient_lists_are_matched_per_stream_user_workspace() {
        let member = Uuid::new_v4();
        let outsider = Uuid::new_v4();
        let acl = PermissionFlagAcl::default();

        assert!(!broadcast_event(Some(Vec::new()), None).may_reach(true, Some(member), &acl));
        assert!(broadcast_event(Some(vec![member]), None).may_reach(true, Some(member), &acl));
        assert!(!broadcast_event(Some(vec![member]), None).may_reach(true, Some(outsider), &acl));
        assert!(
            broadcast_event(Some(vec![member, outsider]), None).may_reach(
                true,
                Some(outsider),
                &acl
            )
        );
        assert!(!broadcast_event(Some(vec![member]), None).may_reach(true, None, &acl));
    }

    #[test]
    fn morph_relation_duplicates_are_collapsed_into_head_events() {
        let heads = HashMap::from([
            (
                String::from("head-1"),
                json!({"id": "head-1", "type": "MORPH_RELATION", "universalIdentifier": "head-1"}),
            ),
            (
                String::from("head-2"),
                json!({"id": "head-2", "type": "MORPH_RELATION", "universalIdentifier": "head-2"}),
            ),
        ]);
        let morph_target = |record_id: &str, morph_id: &str| {
            event(
                MetadataEventType::Created,
                record_id,
                json!({"after": {
                    "id": record_id,
                    "type": "MORPH_RELATION",
                    "morphId": morph_id,
                    "universalIdentifier": record_id
                }}),
            )
        };
        let events = vec![
            event(
                MetadataEventType::Updated,
                "field-plain",
                json!({"after": {"id": "field-plain", "type": "RELATION"}}),
            ),
            morph_target("field-target-a", "head-1"),
            event(
                MetadataEventType::Updated,
                "head-2",
                json!({"after": {"id": "head-2", "type": "MORPH_RELATION", "morphId": "head-2", "universalIdentifier": "head-2"}}),
            ),
            morph_target("field-target-b", "head-2"),
            morph_target("field-target-c", "head-1"),
        ];

        let filtered = filter_morph_relation_duplicate_field_events(&events, &heads);
        let record_ids = filtered
            .iter()
            .map(|event| event.record_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(record_ids, vec!["field-plain", "head-2", "head-1"]);
        assert_eq!(filtered[2].event_type, MetadataEventType::Updated);
        assert_eq!(filtered[2].properties["updatedFields"], json!([]));
        assert_eq!(filtered[2].properties["after"]["id"], json!("head-1"));

        let resolved = resolve_broadcast_events(
            "fieldMetadata",
            &filtered,
            &MetadataBroadcastContext::workspace_wide().with_morph_relation_heads(Some(&heads)),
            Some("hash-1"),
        );
        assert_eq!(resolved.len(), 3);
        assert_eq!(
            resolved[2].updated_collection_hash.as_deref(),
            Some("hash-1")
        );
        assert_eq!(resolved[2].entity_name, "fieldMetadata");
    }

    #[test]
    fn metadata_event_names_follow_the_upstream_composition() {
        assert_eq!(
            compute_metadata_event_name("workflow", MetadataEventType::Updated),
            "metadata.workflow.updated"
        );
        assert_eq!(
            MetadataEventType::parse("deleted"),
            Some(MetadataEventType::Deleted)
        );
        assert_eq!(MetadataEventType::parse("restored"), None);
    }
}
