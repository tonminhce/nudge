#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

use serde::de::Error as DeError;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

use crate::metadata::flat::UniversalFlatFieldMetadata;
use crate::metadata::flat::UniversalFlatIndexMetadata;
use crate::metadata::flat::UniversalFlatObjectMetadata;
use crate::metadata::flat::UniversalFlatSearchFieldMetadata;

pub const CREATE: &str = "create";
pub const UPDATE: &str = "update";
pub const DELETE: &str = "delete";

const ALL_METADATA_NAME_KEYS: &[&str] = &[
    "fieldMetadata",
    "fieldMetadataOption",
    "objectMetadata",
    "view",
    "viewField",
    "viewFieldGroup",
    "viewGroup",
    "viewSort",
    "rowLevelPermissionPredicate",
    "rowLevelPermissionPredicateGroup",
    "rowLevelPermissionRule",
    "viewFilterGroup",
    "index",
    "logicFunction",
    "viewFilter",
    "role",
    "roleTarget",
    "agent",
    "skill",
    "pageLayout",
    "pageLayoutWidget",
    "pageLayoutTab",
    "commandMenuItem",
    "navigationMenuItem",
    "rolePermissionFlag",
    "permissionFlag",
    "objectPermission",
    "fieldPermission",
    "frontComponent",
    "webhook",
    "applicationVariable",
    "connectionProvider",
    "messagingProvider",
    "mcpServer",
    "searchFieldMetadata",
    "timelineActivityType",
    "settingsMenuItem",
    "relatedRecordPath",
    "validationRule",
    "workflow",
    "workflowVersion",
    "agentChatChannel",
    "agentChatChannelRole",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MetadataName {
    #[default]
    FieldMetadata,
    FieldMetadataOption,
    ObjectMetadata,
    View,
    ViewField,
    ViewFieldGroup,
    ViewGroup,
    ViewSort,
    RowLevelPermissionPredicate,
    RowLevelPermissionPredicateGroup,
    RowLevelPermissionRule,
    ViewFilterGroup,
    Index,
    LogicFunction,
    ViewFilter,
    Role,
    RoleTarget,
    Agent,
    Skill,
    PageLayout,
    PageLayoutWidget,
    PageLayoutTab,
    CommandMenuItem,
    NavigationMenuItem,
    RolePermissionFlag,
    PermissionFlag,
    ObjectPermission,
    FieldPermission,
    FrontComponent,
    Webhook,
    ApplicationVariable,
    ConnectionProvider,
    MessagingProvider,
    McpServer,
    SearchFieldMetadata,
    TimelineActivityType,
    SettingsMenuItem,
    RelatedRecordPath,
    ValidationRule,
    Workflow,
    WorkflowVersion,
    AgentChatChannel,
    AgentChatChannelRole,
}

impl MetadataName {
    pub fn as_str(self) -> &'static str {
        match self {
            MetadataName::FieldMetadata => "fieldMetadata",
            MetadataName::FieldMetadataOption => "fieldMetadataOption",
            MetadataName::ObjectMetadata => "objectMetadata",
            MetadataName::View => "view",
            MetadataName::ViewField => "viewField",
            MetadataName::ViewFieldGroup => "viewFieldGroup",
            MetadataName::ViewGroup => "viewGroup",
            MetadataName::ViewSort => "viewSort",
            MetadataName::RowLevelPermissionPredicate => "rowLevelPermissionPredicate",
            MetadataName::RowLevelPermissionPredicateGroup => "rowLevelPermissionPredicateGroup",
            MetadataName::RowLevelPermissionRule => "rowLevelPermissionRule",
            MetadataName::ViewFilterGroup => "viewFilterGroup",
            MetadataName::Index => "index",
            MetadataName::LogicFunction => "logicFunction",
            MetadataName::ViewFilter => "viewFilter",
            MetadataName::Role => "role",
            MetadataName::RoleTarget => "roleTarget",
            MetadataName::Agent => "agent",
            MetadataName::Skill => "skill",
            MetadataName::PageLayout => "pageLayout",
            MetadataName::PageLayoutWidget => "pageLayoutWidget",
            MetadataName::PageLayoutTab => "pageLayoutTab",
            MetadataName::CommandMenuItem => "commandMenuItem",
            MetadataName::NavigationMenuItem => "navigationMenuItem",
            MetadataName::RolePermissionFlag => "rolePermissionFlag",
            MetadataName::PermissionFlag => "permissionFlag",
            MetadataName::ObjectPermission => "objectPermission",
            MetadataName::FieldPermission => "fieldPermission",
            MetadataName::FrontComponent => "frontComponent",
            MetadataName::Webhook => "webhook",
            MetadataName::ApplicationVariable => "applicationVariable",
            MetadataName::ConnectionProvider => "connectionProvider",
            MetadataName::MessagingProvider => "messagingProvider",
            MetadataName::McpServer => "mcpServer",
            MetadataName::SearchFieldMetadata => "searchFieldMetadata",
            MetadataName::TimelineActivityType => "timelineActivityType",
            MetadataName::SettingsMenuItem => "settingsMenuItem",
            MetadataName::RelatedRecordPath => "relatedRecordPath",
            MetadataName::ValidationRule => "validationRule",
            MetadataName::Workflow => "workflow",
            MetadataName::WorkflowVersion => "workflowVersion",
            MetadataName::AgentChatChannel => "agentChatChannel",
            MetadataName::AgentChatChannelRole => "agentChatChannelRole",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        MetadataName::from_key(raw)
    }

    fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "fieldMetadata" => MetadataName::FieldMetadata,
            "fieldMetadataOption" => MetadataName::FieldMetadataOption,
            "objectMetadata" => MetadataName::ObjectMetadata,
            "view" => MetadataName::View,
            "viewField" => MetadataName::ViewField,
            "viewFieldGroup" => MetadataName::ViewFieldGroup,
            "viewGroup" => MetadataName::ViewGroup,
            "viewSort" => MetadataName::ViewSort,
            "rowLevelPermissionPredicate" => MetadataName::RowLevelPermissionPredicate,
            "rowLevelPermissionPredicateGroup" => MetadataName::RowLevelPermissionPredicateGroup,
            "rowLevelPermissionRule" => MetadataName::RowLevelPermissionRule,
            "viewFilterGroup" => MetadataName::ViewFilterGroup,
            "index" => MetadataName::Index,
            "logicFunction" => MetadataName::LogicFunction,
            "viewFilter" => MetadataName::ViewFilter,
            "role" => MetadataName::Role,
            "roleTarget" => MetadataName::RoleTarget,
            "agent" => MetadataName::Agent,
            "skill" => MetadataName::Skill,
            "pageLayout" => MetadataName::PageLayout,
            "pageLayoutWidget" => MetadataName::PageLayoutWidget,
            "pageLayoutTab" => MetadataName::PageLayoutTab,
            "commandMenuItem" => MetadataName::CommandMenuItem,
            "navigationMenuItem" => MetadataName::NavigationMenuItem,
            "rolePermissionFlag" => MetadataName::RolePermissionFlag,
            "permissionFlag" => MetadataName::PermissionFlag,
            "objectPermission" => MetadataName::ObjectPermission,
            "fieldPermission" => MetadataName::FieldPermission,
            "frontComponent" => MetadataName::FrontComponent,
            "webhook" => MetadataName::Webhook,
            "applicationVariable" => MetadataName::ApplicationVariable,
            "connectionProvider" => MetadataName::ConnectionProvider,
            "messagingProvider" => MetadataName::MessagingProvider,
            "mcpServer" => MetadataName::McpServer,
            "searchFieldMetadata" => MetadataName::SearchFieldMetadata,
            "timelineActivityType" => MetadataName::TimelineActivityType,
            "settingsMenuItem" => MetadataName::SettingsMenuItem,
            "relatedRecordPath" => MetadataName::RelatedRecordPath,
            "validationRule" => MetadataName::ValidationRule,
            "workflow" => MetadataName::Workflow,
            "workflowVersion" => MetadataName::WorkflowVersion,
            "agentChatChannel" => MetadataName::AgentChatChannel,
            "agentChatChannelRole" => MetadataName::AgentChatChannelRole,
            _ => return None,
        })
    }

    pub fn all() -> &'static [MetadataName] {
        static ALL: std::sync::OnceLock<Vec<MetadataName>> = std::sync::OnceLock::new();

        ALL.get_or_init(|| {
            ALL_METADATA_NAME_KEYS
                .iter()
                .filter_map(|key| MetadataName::from_key(key))
                .collect()
        })
    }
}

impl fmt::Display for MetadataName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for MetadataName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MetadataName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;

        MetadataName::parse(&raw)
            .ok_or_else(|| DeError::custom(format!("unknown metadata name \"{raw}\"")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceMigrationActionType {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "delete")]
    Delete,
}

impl WorkspaceMigrationActionType {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkspaceMigrationActionType::Create => CREATE,
            WorkspaceMigrationActionType::Update => UPDATE,
            WorkspaceMigrationActionType::Delete => DELETE,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UniversalFlatEntity {
    ObjectMetadata(Box<UniversalFlatObjectMetadata>),
    FieldMetadata(Box<UniversalFlatFieldMetadata>),
    IndexMetadata(Box<UniversalFlatIndexMetadata>),
    FieldMetadataOption(Box<crate::metadata::flat::UniversalFlatFieldMetadataOption>),
    SearchFieldMetadata(Box<UniversalFlatSearchFieldMetadata>),
    Other {
        metadata_name: MetadataName,
        entity: serde_json::Value,
    },
}

impl UniversalFlatEntity {
    pub fn metadata_name(&self) -> MetadataName {
        match self {
            UniversalFlatEntity::ObjectMetadata(_) => MetadataName::ObjectMetadata,
            UniversalFlatEntity::FieldMetadata(_) => MetadataName::FieldMetadata,
            UniversalFlatEntity::IndexMetadata(_) => MetadataName::Index,
            UniversalFlatEntity::FieldMetadataOption(_) => MetadataName::FieldMetadataOption,
            UniversalFlatEntity::SearchFieldMetadata(_) => MetadataName::SearchFieldMetadata,
            UniversalFlatEntity::Other { metadata_name, .. } => *metadata_name,
        }
    }

    pub fn universal_identifier(&self) -> &str {
        match self {
            UniversalFlatEntity::ObjectMetadata(entity) => &entity.universal_identifier,
            UniversalFlatEntity::FieldMetadata(entity) => &entity.universal_identifier,
            UniversalFlatEntity::IndexMetadata(entity) => &entity.universal_identifier,
            UniversalFlatEntity::FieldMetadataOption(entity) => &entity.universal_identifier,
            UniversalFlatEntity::SearchFieldMetadata(entity) => &entity.universal_identifier,
            UniversalFlatEntity::Other { entity, .. } => entity
                .get("universalIdentifier")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
        }
    }

    pub fn to_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        match self {
            UniversalFlatEntity::ObjectMetadata(entity) => serde_json::to_value(entity),
            UniversalFlatEntity::FieldMetadata(entity) => serde_json::to_value(entity),
            UniversalFlatEntity::IndexMetadata(entity) => serde_json::to_value(entity),
            UniversalFlatEntity::FieldMetadataOption(entity) => serde_json::to_value(entity),
            UniversalFlatEntity::SearchFieldMetadata(entity) => serde_json::to_value(entity),
            UniversalFlatEntity::Other { entity, .. } => Ok(entity.clone()),
        }
    }

    pub fn from_value(
        metadata_name: MetadataName,
        value: serde_json::Value,
    ) -> Result<Self, serde_json::Error> {
        Ok(match metadata_name {
            MetadataName::ObjectMetadata => {
                UniversalFlatEntity::ObjectMetadata(Box::new(serde_json::from_value::<
                    UniversalFlatObjectMetadata,
                >(value)?))
            }
            MetadataName::FieldMetadata => {
                UniversalFlatEntity::FieldMetadata(Box::new(serde_json::from_value::<
                    UniversalFlatFieldMetadata,
                >(value)?))
            }
            MetadataName::Index => {
                UniversalFlatEntity::IndexMetadata(Box::new(serde_json::from_value::<
                    UniversalFlatIndexMetadata,
                >(value)?))
            }
            MetadataName::FieldMetadataOption => {
                UniversalFlatEntity::FieldMetadataOption(Box::new(serde_json::from_value::<
                    crate::metadata::flat::UniversalFlatFieldMetadataOption,
                >(value)?))
            }
            MetadataName::SearchFieldMetadata => {
                UniversalFlatEntity::SearchFieldMetadata(Box::new(serde_json::from_value::<
                    UniversalFlatSearchFieldMetadata,
                >(value)?))
            }
            other => UniversalFlatEntity::Other {
                metadata_name: other,
                entity: value,
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UniversalCreateAction {
    pub metadata_name: MetadataName,
    pub flat_entity: Option<UniversalFlatEntity>,
    pub id: Option<String>,
    pub universal_flat_field_metadatas: Vec<UniversalFlatFieldMetadata>,
    pub field_id_by_universal_identifier: Option<HashMap<String, String>>,
    pub related_universal_flat_field_metadata: Option<UniversalFlatFieldMetadata>,
    pub related_field_id: Option<String>,
    pub column_plan: Option<Vec<String>>,
}

impl UniversalCreateAction {
    pub fn new(metadata_name: MetadataName, flat_entity: UniversalFlatEntity) -> Self {
        Self {
            metadata_name,
            flat_entity: Some(flat_entity),
            ..UniversalCreateAction::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UniversalUpdateAction {
    pub metadata_name: MetadataName,
    pub universal_identifier: String,
    pub update: serde_json::Value,
    pub flat_entity: Option<UniversalFlatEntity>,
    pub diff: Option<serde_json::Value>,
    pub rebuild_search_vector: Option<bool>,
    pub updated_universal_flat_index: Option<UniversalFlatIndexMetadata>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UniversalDeleteAction {
    pub metadata_name: MetadataName,
    pub universal_identifier: String,
    pub flat_entity: Option<UniversalFlatEntity>,
}

#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "the variants mirror the upstream action JSON shapes"
)]
pub enum UniversalAction {
    Create(UniversalCreateAction),
    Update(UniversalUpdateAction),
    Delete(UniversalDeleteAction),
}

impl UniversalAction {
    pub fn action_type(&self) -> WorkspaceMigrationActionType {
        match self {
            UniversalAction::Create(_) => WorkspaceMigrationActionType::Create,
            UniversalAction::Update(_) => WorkspaceMigrationActionType::Update,
            UniversalAction::Delete(_) => WorkspaceMigrationActionType::Delete,
        }
    }

    pub fn metadata_name(&self) -> MetadataName {
        match self {
            UniversalAction::Create(action) => action.metadata_name,
            UniversalAction::Update(action) => action.metadata_name,
            UniversalAction::Delete(action) => action.metadata_name,
        }
    }

    pub fn universal_identifier(&self) -> Option<&str> {
        match self {
            UniversalAction::Create(action) => action
                .flat_entity
                .as_ref()
                .map(|entity| entity.universal_identifier()),
            UniversalAction::Update(action) => Some(&action.universal_identifier),
            UniversalAction::Delete(action) => Some(&action.universal_identifier),
        }
    }

    pub fn into_create(self) -> Option<UniversalCreateAction> {
        match self {
            UniversalAction::Create(action) => Some(action),
            _ => None,
        }
    }

    pub fn into_update(self) -> Option<UniversalUpdateAction> {
        match self {
            UniversalAction::Update(action) => Some(action),
            _ => None,
        }
    }

    pub fn into_delete(self) -> Option<UniversalDeleteAction> {
        match self {
            UniversalAction::Delete(action) => Some(action),
            _ => None,
        }
    }
}

fn serialize_create<S: Serializer>(
    action: &UniversalCreateAction,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut map = serde_json::Map::new();

    map.insert("type".to_string(), serde_json::Value::from(CREATE));
    map.insert(
        "metadataName".to_string(),
        serde_json::Value::from(action.metadata_name.as_str()),
    );

    let flat_entity = action
        .flat_entity
        .as_ref()
        .map(UniversalFlatEntity::to_value)
        .transpose()
        .map_err(serde::ser::Error::custom)?
        .unwrap_or(serde_json::Value::Null);

    map.insert("flatEntity".to_string(), flat_entity);

    if let Some(id) = &action.id {
        map.insert("id".to_string(), serde_json::Value::from(id.as_str()));
    }

    if action.metadata_name == MetadataName::ObjectMetadata {
        map.insert(
            "universalFlatFieldMetadatas".to_string(),
            serde_json::to_value(&action.universal_flat_field_metadatas)
                .map_err(serde::ser::Error::custom)?,
        );

        if let Some(field_id_by_universal_identifier) = &action.field_id_by_universal_identifier {
            map.insert(
                "fieldIdByUniversalIdentifier".to_string(),
                serde_json::to_value(field_id_by_universal_identifier)
                    .map_err(serde::ser::Error::custom)?,
            );
        }
    }

    if action.metadata_name == MetadataName::FieldMetadata {
        if let Some(related) = &action.related_universal_flat_field_metadata {
            map.insert(
                "relatedUniversalFlatFieldMetadata".to_string(),
                serde_json::to_value(related).map_err(serde::ser::Error::custom)?,
            );
        }

        if let Some(related_field_id) = &action.related_field_id {
            map.insert(
                "relatedFieldId".to_string(),
                serde_json::Value::from(related_field_id.as_str()),
            );
        }
    }

    serde_json::Value::Object(map).serialize(serializer)
}

fn deserialize_create(
    value: serde_json::Value,
) -> Result<UniversalCreateAction, serde_json::Error> {
    let object = value
        .as_object()
        .ok_or_else(|| DeError::custom("universal create action must be an object"))?;

    let metadata_name = parse_metadata_name(object, CREATE)?;
    let flat_entity = match object.get("flatEntity") {
        Some(entity_value) if !entity_value.is_null() => Some(UniversalFlatEntity::from_value(
            metadata_name,
            entity_value.clone(),
        )?),
        _ => None,
    };

    let mut action = UniversalCreateAction {
        metadata_name,
        flat_entity,
        id: string_field(object, "id"),
        ..UniversalCreateAction::default()
    };

    if metadata_name == MetadataName::ObjectMetadata {
        if let Some(fields) = object.get("universalFlatFieldMetadatas") {
            action.universal_flat_field_metadatas =
                serde_json::from_value(fields.clone()).map_err(DeError::custom)?;
        }

        if let Some(field_ids) = object.get("fieldIdByUniversalIdentifier") {
            action.field_id_by_universal_identifier =
                serde_json::from_value(field_ids.clone()).map_err(DeError::custom)?;
        }
    }

    if metadata_name == MetadataName::FieldMetadata {
        if let Some(related) = object.get("relatedUniversalFlatFieldMetadata") {
            action.related_universal_flat_field_metadata = Some(
                serde_json::from_value::<UniversalFlatFieldMetadata>(related.clone())
                    .map_err(DeError::custom)?,
            );
        }

        action.related_field_id = string_field(object, "relatedFieldId");
    }

    Ok(action)
}

fn serialize_update<S: Serializer>(
    action: &UniversalUpdateAction,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut map = serde_json::Map::new();

    map.insert("type".to_string(), serde_json::Value::from(UPDATE));
    map.insert(
        "metadataName".to_string(),
        serde_json::Value::from(action.metadata_name.as_str()),
    );
    map.insert(
        "universalIdentifier".to_string(),
        serde_json::Value::from(action.universal_identifier.as_str()),
    );
    map.insert("update".to_string(), action.update.clone());

    if let Some(flat_entity) = &action.flat_entity {
        map.insert(
            "flatEntity".to_string(),
            flat_entity.to_value().map_err(serde::ser::Error::custom)?,
        );
    }

    if let Some(diff) = &action.diff {
        map.insert("diff".to_string(), diff.clone());
    }

    if action.metadata_name == MetadataName::FieldMetadata {
        if let Some(rebuild_search_vector) = action.rebuild_search_vector {
            map.insert(
                "rebuildSearchVector".to_string(),
                serde_json::Value::from(rebuild_search_vector),
            );
        }
    }

    if action.metadata_name == MetadataName::Index {
        if let Some(updated_index) = &action.updated_universal_flat_index {
            map.insert(
                "updatedUniversalFlatIndex".to_string(),
                serde_json::to_value(updated_index).map_err(serde::ser::Error::custom)?,
            );
        }
    }

    serde_json::Value::Object(map).serialize(serializer)
}

fn deserialize_update(
    value: serde_json::Value,
) -> Result<UniversalUpdateAction, serde_json::Error> {
    let object = value
        .as_object()
        .ok_or_else(|| DeError::custom("universal update action must be an object"))?;

    let metadata_name = parse_metadata_name(object, UPDATE)?;
    let universal_identifier = string_field(object, "universalIdentifier")
        .ok_or_else(|| DeError::custom("universal update action requires universalIdentifier"))?;

    let mut action = UniversalUpdateAction {
        metadata_name,
        universal_identifier,
        update: object
            .get("update")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        flat_entity: match object.get("flatEntity") {
            Some(entity_value) if !entity_value.is_null() => Some(UniversalFlatEntity::from_value(
                metadata_name,
                entity_value.clone(),
            )?),
            _ => None,
        },
        diff: object.get("diff").cloned(),
        rebuild_search_vector: None,
        updated_universal_flat_index: None,
    };

    if metadata_name == MetadataName::FieldMetadata {
        action.rebuild_search_vector = bool_field(object, "rebuildSearchVector");
    }

    if metadata_name == MetadataName::Index {
        if let Some(updated_index) = object.get("updatedUniversalFlatIndex") {
            action.updated_universal_flat_index = Some(
                serde_json::from_value::<UniversalFlatIndexMetadata>(updated_index.clone())
                    .map_err(DeError::custom)?,
            );
        }
    }

    Ok(action)
}

fn serialize_delete<S: Serializer>(
    action: &UniversalDeleteAction,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut map = serde_json::Map::new();

    map.insert(
        "universalIdentifier".to_string(),
        serde_json::Value::from(action.universal_identifier.as_str()),
    );
    map.insert("type".to_string(), serde_json::Value::from(DELETE));
    map.insert(
        "metadataName".to_string(),
        serde_json::Value::from(action.metadata_name.as_str()),
    );

    if let Some(flat_entity) = &action.flat_entity {
        map.insert(
            "flatEntity".to_string(),
            flat_entity.to_value().map_err(serde::ser::Error::custom)?,
        );
    }

    serde_json::Value::Object(map).serialize(serializer)
}

fn deserialize_delete(
    value: serde_json::Value,
) -> Result<UniversalDeleteAction, serde_json::Error> {
    let object = value
        .as_object()
        .ok_or_else(|| DeError::custom("universal delete action must be an object"))?;

    let metadata_name = parse_metadata_name(object, DELETE)?;
    let universal_identifier = string_field(object, "universalIdentifier")
        .ok_or_else(|| DeError::custom("universal delete action requires universalIdentifier"))?;

    Ok(UniversalDeleteAction {
        metadata_name,
        universal_identifier,
        flat_entity: match object.get("flatEntity") {
            Some(entity_value) if !entity_value.is_null() => Some(UniversalFlatEntity::from_value(
                metadata_name,
                entity_value.clone(),
            )?),
            _ => None,
        },
    })
}

impl Serialize for UniversalAction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            UniversalAction::Create(action) => serialize_create(action, serializer),
            UniversalAction::Update(action) => serialize_update(action, serializer),
            UniversalAction::Delete(action) => serialize_delete(action, serializer),
        }
    }
}

impl<'de> Deserialize<'de> for UniversalAction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;

        let action_type = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| DeError::custom("universal action requires a type"))?;

        match action_type {
            CREATE => deserialize_create(value)
                .map(UniversalAction::Create)
                .map_err(DeError::custom),
            UPDATE => deserialize_update(value)
                .map(UniversalAction::Update)
                .map_err(DeError::custom),
            DELETE => deserialize_delete(value)
                .map(UniversalAction::Delete)
                .map_err(DeError::custom),
            other => Err(DeError::custom(format!(
                "unknown workspace migration action type \"{other}\""
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceMigration {
    pub actions: Vec<UniversalAction>,
    pub application_universal_identifier: String,
}

impl Serialize for WorkspaceMigration {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serde_json::Map::new();

        map.insert(
            "actions".to_string(),
            serde_json::to_value(&self.actions).map_err(serde::ser::Error::custom)?,
        );
        map.insert(
            "applicationUniversalIdentifier".to_string(),
            serde_json::Value::from(self.application_universal_identifier.as_str()),
        );

        serde_json::Value::Object(map).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WorkspaceMigration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| DeError::custom("workspace migration must be an object"))?;

        let actions = object
            .get("actions")
            .ok_or_else(|| DeError::custom("workspace migration requires actions"))?;

        Ok(WorkspaceMigration {
            actions: serde_json::from_value(actions.clone()).map_err(DeError::custom)?,
            application_universal_identifier: string_field(
                object,
                "applicationUniversalIdentifier",
            )
            .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActionsByOperation {
    pub create: Vec<UniversalAction>,
    pub update: Vec<UniversalAction>,
    pub delete: Vec<UniversalAction>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrchestratorActionsReport {
    pub by_metadata_name: HashMap<MetadataName, ActionsByOperation>,
}

impl OrchestratorActionsReport {
    pub fn actions_for(&self, metadata_name: MetadataName) -> &ActionsByOperation {
        static EMPTY: std::sync::OnceLock<ActionsByOperation> = std::sync::OnceLock::new();

        self.by_metadata_name
            .get(&metadata_name)
            .unwrap_or_else(|| EMPTY.get_or_init(ActionsByOperation::default))
    }

    pub fn actions_for_mut(&mut self, metadata_name: MetadataName) -> &mut ActionsByOperation {
        self.by_metadata_name.entry(metadata_name).or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.by_metadata_name
            .values()
            .all(ActionsByOperation::is_empty)
    }
}

impl ActionsByOperation {
    pub fn is_empty(&self) -> bool {
        self.create.is_empty() && self.update.is_empty() && self.delete.is_empty()
    }

    pub fn push(&mut self, action: UniversalAction) {
        match action.action_type() {
            WorkspaceMigrationActionType::Create => self.create.push(action),
            WorkspaceMigrationActionType::Update => self.update.push(action),
            WorkspaceMigrationActionType::Delete => self.delete.push(action),
        }
    }
}

fn parse_metadata_name(
    object: &serde_json::Map<String, serde_json::Value>,
    action_type: &str,
) -> Result<MetadataName, serde_json::Error> {
    let raw = object
        .get("metadataName")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            serde_json::Error::custom(format!("{action_type} action requires metadataName"))
        })?;

    MetadataName::parse(raw)
        .ok_or_else(|| serde_json::Error::custom(format!("unknown metadata name \"{raw}\"")))
}

fn string_field(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn bool_field(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<bool> {
    object.get(key).and_then(serde_json::Value::as_bool)
}

pub const OBJECT_COMPARABLE_PROPERTIES: &[&str] = &[
    "openRecordIn",
    "color",
    "description",
    "icon",
    "isActive",
    "isLabelSyncedWithName",
    "labelPlural",
    "labelSingular",
    "namePlural",
    "nameSingular",
    "labelIdentifierFieldMetadataUniversalIdentifier",
    "overrides",
    "isUIEditable",
    "disabledObjectPermissionActions",
    "isUICreatable",
    "isSearchable",
    "duplicateCriteria",
    "imageIdentifierFieldMetadataUniversalIdentifier",
];

pub const FIELD_COMPARABLE_PROPERTIES: &[&str] = &[
    "defaultValue",
    "description",
    "icon",
    "isActive",
    "isLabelSyncedWithName",
    "isUnique",
    "isSearchable",
    "isAuditLogged",
    "label",
    "name",
    "options",
    "overrides",
    "universalSettings",
    "isUIEditable",
    "disabledFieldPermissionActions",
    "isNullable",
];

pub const INDEX_COMPARABLE_PROPERTIES: &[&str] = &[
    "indexType",
    "indexWhereClause",
    "isUnique",
    "name",
    "universalFlatIndexFieldMetadatas",
];

pub const SEARCH_FIELD_COMPARABLE_PROPERTIES: &[&str] = &["position"];

pub const FIELD_OPTION_COMPARABLE_PROPERTIES: &[&str] = &[
    "value",
    "label",
    "color",
    "description",
    "position",
    "isActive",
    "overrides",
];

pub fn comparable_properties(metadata_name: MetadataName) -> &'static [&'static str] {
    match metadata_name {
        MetadataName::ObjectMetadata => OBJECT_COMPARABLE_PROPERTIES,
        MetadataName::FieldMetadata => FIELD_COMPARABLE_PROPERTIES,
        MetadataName::Index => INDEX_COMPARABLE_PROPERTIES,
        MetadataName::SearchFieldMetadata => SEARCH_FIELD_COMPARABLE_PROPERTIES,
        MetadataName::FieldMetadataOption => FIELD_OPTION_COMPARABLE_PROPERTIES,
        _ => &[],
    }
}

pub fn sanitize_universal_flat_entity_update(
    metadata_name: MetadataName,
    update: &serde_json::Value,
) -> serde_json::Value {
    let comparable = comparable_properties(metadata_name);
    let mut sanitized = serde_json::Map::new();

    if let Some(object) = update.as_object() {
        for property in comparable {
            if let Some(value) = object.get(*property) {
                sanitized.insert((*property).to_string(), value.clone());
            }
        }
    }

    serde_json::Value::Object(sanitized)
}

pub fn diff_universal_flat_entity_properties(
    metadata_name: MetadataName,
    from_entity: &serde_json::Value,
    to_entity: &serde_json::Value,
) -> serde_json::Value {
    let comparable = comparable_properties(metadata_name);
    let mut update = serde_json::Map::new();

    for property in comparable {
        let to_value = match to_entity.get(*property) {
            Some(value) => value,
            None => continue,
        };

        let from_value = from_entity
            .get(*property)
            .unwrap_or(&serde_json::Value::Null);

        if from_value != to_value {
            update.insert((*property).to_string(), to_value.clone());
        }
    }

    serde_json::Value::Object(update)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::MetadataName;
    use super::UniversalAction;
    use super::UniversalCreateAction;
    use super::UniversalDeleteAction;
    use super::UniversalFlatEntity;
    use super::UniversalUpdateAction;
    use super::WorkspaceMigration;
    use crate::metadata::flat::UniversalFlatObjectMetadata;

    fn object_entity(universal_identifier: &str) -> UniversalFlatEntity {
        let sample_timestamp =
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2024, 1, 1, 0, 0, 0).unwrap();

        UniversalFlatEntity::ObjectMetadata(Box::new(UniversalFlatObjectMetadata {
            universal_identifier: universal_identifier.to_string(),
            name_singular: "rocket".to_string(),
            name_plural: "rockets".to_string(),
            label_singular: "Rocket".to_string(),
            label_plural: "Rockets".to_string(),
            application_universal_identifier: "6c5847b1-0ff4-4bd9-9b4c-6b1f2a264264".to_string(),
            created_at: sample_timestamp,
            updated_at: sample_timestamp,
            ..UniversalFlatObjectMetadata::default()
        }))
    }

    #[test]
    fn metadata_name_round_trips_every_name() {
        for metadata_name in MetadataName::all() {
            assert_eq!(
                MetadataName::parse(metadata_name.as_str()),
                Some(*metadata_name)
            );
        }
    }

    #[test]
    fn workspace_migration_serializes_upstream_envelope() {
        let migration = WorkspaceMigration {
            actions: vec![UniversalAction::Create(UniversalCreateAction {
                metadata_name: MetadataName::ObjectMetadata,
                flat_entity: Some(object_entity("20202020-e674-48e5-a542-72570eee7213")),
                universal_flat_field_metadatas: Vec::new(),
                ..UniversalCreateAction::default()
            })],
            application_universal_identifier: "6c5847b1-0ff4-4bd9-9b4c-6b1f2a264264".to_string(),
        };

        let value = serde_json::to_value(&migration).unwrap();

        assert_eq!(
            value
                .get("applicationUniversalIdentifier")
                .and_then(|raw| raw.as_str()),
            Some("6c5847b1-0ff4-4bd9-9b4c-6b1f2a264264")
        );

        let actions = value.get("actions").and_then(|raw| raw.as_array()).unwrap();
        let create_action = &actions[0];

        assert_eq!(
            create_action.get("type").and_then(|raw| raw.as_str()),
            Some("create")
        );
        assert_eq!(
            create_action
                .get("metadataName")
                .and_then(|raw| raw.as_str()),
            Some("objectMetadata")
        );
        assert_eq!(
            create_action.get("universalFlatFieldMetadatas"),
            Some(&json!([]))
        );
        assert_eq!(
            create_action
                .get("flatEntity")
                .and_then(|entity| entity.get("universalIdentifier"))
                .and_then(|raw| raw.as_str()),
            Some("20202020-e674-48e5-a542-72570eee7213")
        );

        let deserialized: WorkspaceMigration = serde_json::from_value(value).unwrap();

        assert_eq!(deserialized, migration);
    }

    #[test]
    fn universal_action_round_trips_every_verb() {
        let create = UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::ObjectMetadata,
            flat_entity: Some(object_entity("20202020-e674-48e5-a542-72570eee7213")),
            id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            universal_flat_field_metadatas: Vec::new(),
            ..UniversalCreateAction::default()
        });

        let update = UniversalAction::Update(UniversalUpdateAction {
            metadata_name: MetadataName::FieldMetadata,
            universal_identifier: "20202020-4d99-46a1-9b8c-33e6c9b0d5c6".to_string(),
            update: json!({ "label": "New label" }),
            rebuild_search_vector: Some(true),
            ..UniversalUpdateAction::default()
        });

        let delete = UniversalAction::Delete(UniversalDeleteAction {
            metadata_name: MetadataName::SearchFieldMetadata,
            universal_identifier: "20202020-b56b-4cf2-c3d4-e5f6a7b8c9d0".to_string(),
            flat_entity: None,
        });

        for action in [create, update, delete] {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: UniversalAction = serde_json::from_str(&serialized).unwrap();

            assert_eq!(deserialized, action);
        }
    }

    #[test]
    fn update_action_carries_rebuild_search_vector_only_for_fields() {
        let update = UniversalAction::Update(UniversalUpdateAction {
            metadata_name: MetadataName::FieldMetadata,
            universal_identifier: "20202020-c67c-4d03-d4e5-f6a7b8c9d0e1".to_string(),
            update: json!({}),
            rebuild_search_vector: Some(true),
            ..UniversalUpdateAction::default()
        });

        let value = serde_json::to_value(&update).unwrap();

        assert_eq!(
            value
                .get("rebuildSearchVector")
                .and_then(|raw| raw.as_bool()),
            Some(true)
        );
        assert_eq!(value.get("update"), Some(&json!({})));
    }

    #[test]
    fn index_update_action_carries_updated_universal_flat_index() {
        let updated_index = crate::metadata::flat::UniversalFlatIndexMetadata {
            universal_identifier: "20202020-cbf8-4df4-b19d-37ef05946c6d".to_string(),
            name: "IDX_ROCKETS".to_string(),
            object_metadata_universal_identifier: "20202020-e674-48e5-a542-72570eee7213"
                .to_string(),
            is_unique: true,
            ..crate::metadata::flat::UniversalFlatIndexMetadata::default()
        };

        let update = UniversalAction::Update(UniversalUpdateAction {
            metadata_name: MetadataName::Index,
            universal_identifier: "20202020-cbf8-4df4-b19d-37ef05946c6d".to_string(),
            update: json!({ "isUnique": true }),
            updated_universal_flat_index: Some(updated_index),
            ..UniversalUpdateAction::default()
        });

        let serialized = serde_json::to_string(&update).unwrap();
        let deserialized: UniversalAction = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, update);
    }

    #[test]
    fn generic_row_action_round_trips_for_non_owned_metadata_names() {
        let create = UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::Webhook,
            flat_entity: Some(UniversalFlatEntity::Other {
                metadata_name: MetadataName::Webhook,
                entity: json!({
                    "universalIdentifier": "20202020-f1e2-4a33-8b4c-5d6e7f8a9b0c",
                    "targetUrl": "https://example.com/hook"
                }),
            }),
            ..UniversalCreateAction::default()
        });

        let serialized = serde_json::to_string(&create).unwrap();
        let deserialized: UniversalAction = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, create);
    }

    #[test]
    fn sanitize_update_keeps_only_comparable_properties() {
        let update = json!({
            "label": "New",
            "isSystem": true,
            "type": "TEXT",
            "isActive": false
        });

        let sanitized =
            super::sanitize_universal_flat_entity_update(MetadataName::FieldMetadata, &update);

        assert_eq!(sanitized, json!({ "label": "New", "isActive": false }));
    }

    #[test]
    fn diff_returns_changed_properties_only() {
        let from = json!({
            "label": "Old",
            "name": "field",
            "isNullable": true
        });
        let to = json!({
            "label": "New",
            "name": "field",
            "isNullable": false
        });

        let diff =
            super::diff_universal_flat_entity_properties(MetadataName::FieldMetadata, &from, &to);

        assert_eq!(diff, json!({ "label": "New", "isNullable": false }));
    }
}
