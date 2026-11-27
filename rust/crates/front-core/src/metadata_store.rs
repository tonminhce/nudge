use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

pub const UPSTREAM_TAG: &str = "twenty/v2.61.0";
pub const METADATA_ENTITY_KEY_COUNT: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetadataEntityKey {
    ObjectMetadataItems,
    FieldMetadataItems,
    IndexMetadataItems,
    Views,
    ViewFields,
    ViewFilters,
    ViewSorts,
    ViewGroups,
    ViewFilterGroups,
    ViewFieldGroups,
    PageLayouts,
    PageLayoutTabs,
    PageLayoutWidgets,
    LogicFunctions,
    NavigationMenuItems,
    CommandMenuItems,
    FrontComponents,
    Webhooks,
    Roles,
    RoleTargets,
    Agents,
    Skills,
    RowLevelPermissionPredicates,
    RowLevelPermissionPredicateGroups,
    Applications,
}

impl MetadataEntityKey {
    pub const ALL: [Self; METADATA_ENTITY_KEY_COUNT] = [
        Self::ObjectMetadataItems,
        Self::FieldMetadataItems,
        Self::IndexMetadataItems,
        Self::Views,
        Self::ViewFields,
        Self::ViewFilters,
        Self::ViewSorts,
        Self::ViewGroups,
        Self::ViewFilterGroups,
        Self::ViewFieldGroups,
        Self::PageLayouts,
        Self::PageLayoutTabs,
        Self::PageLayoutWidgets,
        Self::LogicFunctions,
        Self::NavigationMenuItems,
        Self::CommandMenuItems,
        Self::FrontComponents,
        Self::Webhooks,
        Self::Roles,
        Self::RoleTargets,
        Self::Agents,
        Self::Skills,
        Self::RowLevelPermissionPredicates,
        Self::RowLevelPermissionPredicateGroups,
        Self::Applications,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MetadataLoadStatus {
    #[default]
    Idle,
    Loading,
    Current,
    Stale,
}

#[derive(Debug, Clone, Default)]
pub struct MetadataEntityState {
    pub current: Vec<Value>,
    pub draft: Vec<Value>,
    pub status: MetadataLoadStatus,
    pub current_collection_hash: Option<String>,
    pub draft_collection_hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinimalObjectMetadata {
    pub id: String,
    pub name_singular: String,
    pub name_plural: String,
    pub label_singular: String,
    pub label_plural: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub is_active: bool,
    pub is_system: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinimalView {
    pub id: String,
    #[serde(rename = "type")]
    pub view_type: String,
    #[serde(default)]
    pub key: Option<String>,
    pub object_metadata_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionHash {
    pub collection_name: String,
    pub hash: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindMinimalMetadata {
    pub object_metadata_items: Vec<MinimalObjectMetadata>,
    pub views: Vec<MinimalView>,
    pub collection_hashes: Vec<CollectionHash>,
}

impl FindMinimalMetadata {
    pub fn from_response(data: &Value) -> Result<Self, serde_json::Error> {
        let payload = data
            .get("minimalMetadata")
            .or_else(|| {
                data.get("data")
                    .and_then(|data| data.get("minimalMetadata"))
            })
            .cloned()
            .unwrap_or(Value::Null);
        serde_json::from_value(payload)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MetadataStore {
    entities: HashMap<MetadataEntityKey, MetadataEntityState>,
}

impl MetadataStore {
    pub fn new() -> Self {
        Self {
            entities: MetadataEntityKey::ALL
                .into_iter()
                .map(|key| (key, MetadataEntityState::default()))
                .collect(),
        }
    }

    pub fn state(&self, key: MetadataEntityKey) -> Option<&MetadataEntityState> {
        self.entities.get(&key)
    }

    pub fn state_mut(&mut self, key: MetadataEntityKey) -> Option<&mut MetadataEntityState> {
        self.entities.get_mut(&key)
    }

    pub fn current(&self, key: MetadataEntityKey) -> &[Value] {
        self.entities
            .get(&key)
            .map(|state| state.current.as_slice())
            .unwrap_or_default()
    }

    pub fn set_loading(&mut self, key: MetadataEntityKey) {
        if let Some(state) = self.entities.get_mut(&key) {
            state.status = MetadataLoadStatus::Loading;
        }
    }

    pub fn hydrate_minimal(&mut self, payload: FindMinimalMetadata) {
        if let Some(state) = self
            .entities
            .get_mut(&MetadataEntityKey::ObjectMetadataItems)
        {
            state.current = serde_json::to_value(payload.object_metadata_items)
                .ok()
                .as_ref()
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            state.status = if state.current.is_empty() {
                MetadataLoadStatus::Idle
            } else {
                MetadataLoadStatus::Current
            };
        }

        if let Some(state) = self.entities.get_mut(&MetadataEntityKey::Views) {
            state.current = serde_json::to_value(payload.views)
                .ok()
                .as_ref()
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            state.status = if state.current.is_empty() {
                MetadataLoadStatus::Idle
            } else {
                MetadataLoadStatus::Current
            };
        }

        for hash in payload.collection_hashes {
            if let Some(key) = MetadataEntityKey::from_collection_name(&hash.collection_name) {
                if let Some(state) = self.entities.get_mut(&key) {
                    state.current_collection_hash = Some(hash.hash);
                }
            }
        }
    }

    pub fn apply_full_load(
        &mut self,
        key: MetadataEntityKey,
        items: Vec<Value>,
        collection_hash: Option<String>,
    ) {
        let status = MetadataLoadStatus::Current;
        if let Some(state) = self.entities.get_mut(&key) {
            state.current = items;
            state.status = status;
            if let Some(hash) = collection_hash {
                state.current_collection_hash = Some(hash);
            }
        }
    }

    pub fn is_stale(&self, key: MetadataEntityKey, hash: &str) -> bool {
        self.entities
            .get(&key)
            .map(|state| {
                state
                    .current_collection_hash
                    .as_deref()
                    .is_none_or(|current| current != hash)
            })
            .unwrap_or(true)
    }

    pub fn gate_is_open(&self) -> bool {
        !self
            .current(MetadataEntityKey::ObjectMetadataItems)
            .is_empty()
            && !self.current(MetadataEntityKey::Views).is_empty()
    }
}

impl MetadataEntityKey {
    pub fn from_collection_name(name: &str) -> Option<Self> {
        match name {
            "objectMetadata" | "objectMetadataItems" => Some(Self::ObjectMetadataItems),
            "fieldMetadata" | "fieldMetadataItems" => Some(Self::FieldMetadataItems),
            "index" | "indexMetadataItems" => Some(Self::IndexMetadataItems),
            "view" | "views" => Some(Self::Views),
            "viewField" | "viewFields" => Some(Self::ViewFields),
            "viewFilter" | "viewFilters" => Some(Self::ViewFilters),
            "viewSort" | "viewSorts" => Some(Self::ViewSorts),
            "viewGroup" | "viewGroups" => Some(Self::ViewGroups),
            "viewFilterGroup" | "viewFilterGroups" => Some(Self::ViewFilterGroups),
            "viewFieldGroup" | "viewFieldGroups" => Some(Self::ViewFieldGroups),
            "pageLayout" | "pageLayouts" => Some(Self::PageLayouts),
            "pageLayoutTab" | "pageLayoutTabs" => Some(Self::PageLayoutTabs),
            "pageLayoutWidget" | "pageLayoutWidgets" => Some(Self::PageLayoutWidgets),
            "logicFunction" | "logicFunctions" => Some(Self::LogicFunctions),
            "navigationMenuItem" | "navigationMenuItems" => Some(Self::NavigationMenuItems),
            "commandMenuItem" | "commandMenuItems" => Some(Self::CommandMenuItems),
            "frontComponent" | "frontComponents" => Some(Self::FrontComponents),
            "webhook" | "webhooks" => Some(Self::Webhooks),
            "role" | "roles" => Some(Self::Roles),
            "roleTarget" | "roleTargets" => Some(Self::RoleTargets),
            "agent" | "agents" => Some(Self::Agents),
            "skill" | "skills" => Some(Self::Skills),
            "rowLevelPermissionPredicate" | "rowLevelPermissionPredicates" => {
                Some(Self::RowLevelPermissionPredicates)
            }
            "rowLevelPermissionPredicateGroup" | "rowLevelPermissionPredicateGroups" => {
                Some(Self::RowLevelPermissionPredicateGroups)
            }
            "application" | "applications" => Some(Self::Applications),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal_payload() -> FindMinimalMetadata {
        serde_json::from_value(json!({
            "objectMetadataItems": [{
                "id": "person",
                "nameSingular": "person",
                "namePlural": "people",
                "labelSingular": "Person",
                "labelPlural": "People",
                "icon": "IconUser",
                "isActive": true,
                "isSystem": false,
                "isRemote": false
            }],
            "views": [{
                "id": "all-people",
                "type": "TABLE",
                "key": "INDEX",
                "objectMetadataId": "person"
            }],
            "collectionHashes": [{"collectionName": "objectMetadataItems", "hash": "h1"}]
        }))
        .unwrap()
    }

    #[test]
    fn metadata_store_tracks_the_upstream_twenty_five_entity_kinds() {
        assert_eq!(MetadataEntityKey::ALL.len(), 25);
        let store = MetadataStore::new();
        for key in MetadataEntityKey::ALL {
            assert!(store.state(key).is_some());
        }
    }

    #[test]
    fn hydrate_minimal_opens_the_gate_with_objects_and_views() {
        let mut store = MetadataStore::new();
        assert!(!store.gate_is_open());
        store.hydrate_minimal(minimal_payload());
        assert!(store.gate_is_open());
        assert_eq!(
            store.current(MetadataEntityKey::ObjectMetadataItems).len(),
            1
        );
        assert_eq!(store.current(MetadataEntityKey::Views).len(), 1);
    }

    #[test]
    fn collection_hash_diff_marks_the_reference_kind_stale() {
        let mut store = MetadataStore::new();
        store.hydrate_minimal(minimal_payload());
        assert!(!store.is_stale(MetadataEntityKey::ObjectMetadataItems, "h1"));
        assert!(store.is_stale(MetadataEntityKey::ObjectMetadataItems, "h2"));
    }

    #[test]
    fn upstream_collection_names_map_to_the_frontend_entity_kinds() {
        assert_eq!(
            MetadataEntityKey::from_collection_name("objectMetadata"),
            Some(MetadataEntityKey::ObjectMetadataItems)
        );
        assert_eq!(
            MetadataEntityKey::from_collection_name("fieldMetadata"),
            Some(MetadataEntityKey::FieldMetadataItems)
        );
        assert_eq!(
            MetadataEntityKey::from_collection_name("view"),
            Some(MetadataEntityKey::Views)
        );
        assert_eq!(
            MetadataEntityKey::from_collection_name("index"),
            Some(MetadataEntityKey::IndexMetadataItems)
        );
        assert_eq!(
            MetadataEntityKey::from_collection_name("rolePermissionFlag"),
            None
        );
    }

    #[test]
    fn full_load_replaces_the_current_collection() {
        let mut store = MetadataStore::new();
        store.apply_full_load(
            MetadataEntityKey::CommandMenuItems,
            vec![json!({"id": "search"})],
            Some("hash-1".into()),
        );
        assert_eq!(store.current(MetadataEntityKey::CommandMenuItems).len(), 1);
        assert_eq!(
            store
                .state(MetadataEntityKey::CommandMenuItems)
                .unwrap()
                .current_collection_hash
                .as_deref(),
            Some("hash-1")
        );
    }
}
