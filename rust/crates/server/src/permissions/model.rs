#![allow(dead_code)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;

use serde::Serialize;
use uuid::Uuid;

pub const CREATE_RECORDS_PERMISSION_ENABLED: bool = false;

pub const ALL_PERMISSION_FLAGS: &[&str] = &[
    "API_KEYS_AND_WEBHOOKS",
    "WORKSPACE",
    "WORKSPACE_MEMBERS",
    "ROLES",
    "DATA_MODEL",
    "SECURITY",
    "WORKFLOWS",
    "IMPERSONATE",
    "SSO_BYPASS",
    "APPLICATIONS",
    "MARKETPLACE_APPS",
    "LAYOUTS",
    "BILLING",
    "AI_SETTINGS",
    "AI",
    "VIEWS",
    "UPLOAD_FILE",
    "DOWNLOAD_FILE",
    "SEND_EMAIL_TOOL",
    "CREATE_CALENDAR_EVENT_TOOL",
    "HTTP_REQUEST_TOOL",
    "CODE_INTERPRETER_TOOL",
    "IMPORT_CSV",
    "EXPORT_CSV",
    "CONNECTED_ACCOUNTS",
    "PROFILE_INFORMATION",
    "READ_PRIVATE_CHANNELS",
];

pub const TOOL_PERMISSION_FLAGS: &[&str] = &[
    "CODE_INTERPRETER_TOOL",
    "CREATE_CALENDAR_EVENT_TOOL",
    "HTTP_REQUEST_TOOL",
    "SEND_EMAIL_TOOL",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectRecordsOperationType {
    Create,
    Read,
    Update,
    SoftDelete,
    Destroy,
}

impl ObjectRecordsOperationType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Read => "read",
            Self::Update => "update",
            Self::SoftDelete => "softDelete",
            Self::Destroy => "destroy",
        }
    }

    pub fn enforced(self) -> Self {
        if self == Self::Create && !CREATE_RECORDS_PERMISSION_ENABLED {
            Self::Update
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPrincipalKind {
    WorkspaceMember,
    ApiKey,
    Agent,
    Application,
    System,
}

#[derive(Debug, Clone)]
pub struct ObjectPermissionDefinition {
    pub object_metadata_id: Uuid,
    pub can_read_object_records: Option<bool>,
    pub can_update_object_records: Option<bool>,
    pub can_soft_delete_object_records: Option<bool>,
    pub can_destroy_object_records: Option<bool>,
    pub can_create_object_records: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct FieldPermissionDefinition {
    pub object_metadata_id: Uuid,
    pub field_metadata_id: Uuid,
    pub can_read_field_value: Option<bool>,
    pub can_update_field_value: Option<bool>,
    pub row_level_permission_rule_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct RowLevelPermissionRuleDefinition {
    pub id: Uuid,
    pub object_metadata_id: Uuid,
    pub can_create_records: bool,
    pub can_read_records: bool,
    pub can_update_records: bool,
    pub can_soft_delete_records: bool,
    pub can_destroy_records: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowLevelPermissionPredicateDefinition {
    pub id: Uuid,
    pub field_metadata_id: Uuid,
    pub relation_field_metadata_id: Option<Uuid>,
    pub object_metadata_id: Uuid,
    pub operand: String,
    pub sub_field_name: Option<String>,
    pub workspace_member_field_metadata_id: Option<Uuid>,
    pub workspace_member_sub_field_name: Option<String>,
    pub workspace_member_relation_field_metadata_id: Option<Uuid>,
    pub row_level_permission_predicate_group_id: Option<Uuid>,
    pub position_in_row_level_permission_predicate_group: Option<f64>,
    pub role_id: Uuid,
    pub value: Option<serde_json::Value>,
    pub row_level_permission_rule_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowLevelPermissionPredicateGroupDefinition {
    pub id: Uuid,
    pub parent_row_level_permission_predicate_group_id: Option<Uuid>,
    pub logical_operator: String,
    pub position_in_row_level_permission_predicate_group: Option<f64>,
    pub can_read_records: bool,
    pub can_update_records: bool,
    pub can_soft_delete_records: bool,
    pub can_destroy_records: bool,
    pub role_id: Uuid,
    pub object_metadata_id: Uuid,
    pub row_level_permission_rule_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct RoleDefinition {
    pub id: Uuid,
    pub can_update_all_settings: bool,
    pub can_access_all_tools: bool,
    pub can_read_all_object_records: bool,
    pub can_update_all_object_records: bool,
    pub can_soft_delete_all_object_records: bool,
    pub can_destroy_all_object_records: bool,
    pub can_create_all_object_records: bool,
    pub permission_flags: HashSet<String>,
    pub object_permissions: HashMap<Uuid, ObjectPermissionDefinition>,
    pub field_permissions: Vec<FieldPermissionDefinition>,
    pub row_level_permission_rules: Vec<RowLevelPermissionRuleDefinition>,
    pub row_level_permission_predicates: Vec<RowLevelPermissionPredicateDefinition>,
    pub row_level_permission_predicate_groups: Vec<RowLevelPermissionPredicateGroupDefinition>,
}

impl RoleDefinition {
    pub fn empty(id: Uuid) -> Self {
        Self {
            id,
            can_update_all_settings: false,
            can_access_all_tools: false,
            can_read_all_object_records: false,
            can_update_all_object_records: false,
            can_soft_delete_all_object_records: false,
            can_destroy_all_object_records: false,
            can_create_all_object_records: false,
            permission_flags: HashSet::new(),
            object_permissions: HashMap::new(),
            field_permissions: Vec::new(),
            row_level_permission_rules: Vec::new(),
            row_level_permission_predicates: Vec::new(),
            row_level_permission_predicate_groups: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRecordsPermissions {
    pub object_metadata_id: String,
    pub can_create_all_records: bool,
    pub can_read_all_records: bool,
    pub can_update_all_records: bool,
    pub can_soft_delete_all_records: bool,
    pub can_destroy_all_records: bool,
    pub unreadable_fields: Vec<String>,
    pub uneditable_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_level_records_permissions: Option<Vec<RowLevelRecordsPermissions>>,
    pub can_read_some_records: bool,
}

impl ObjectRecordsPermissions {
    pub fn operation_allowed(&self, operation: ObjectRecordsOperationType) -> bool {
        match operation.enforced() {
            ObjectRecordsOperationType::Create => self.can_create_all_records,
            ObjectRecordsOperationType::Read => self.can_read_all_records,
            ObjectRecordsOperationType::Update => self.can_update_all_records,
            ObjectRecordsOperationType::SoftDelete => self.can_soft_delete_all_records,
            ObjectRecordsOperationType::Destroy => self.can_destroy_all_records,
        }
    }

    pub fn can_read_some_records(&self) -> bool {
        self.can_access_some_records(ObjectRecordsOperationType::Read)
    }

    pub fn can_access_some_records(&self, operation: ObjectRecordsOperationType) -> bool {
        self.operation_allowed(operation)
            || self
                .row_level_records_permissions
                .as_ref()
                .is_some_and(|rules| {
                    rules.iter().any(|rule| match operation.enforced() {
                        ObjectRecordsOperationType::Create => rule.can_create_records,
                        ObjectRecordsOperationType::Read => rule.can_read_records,
                        ObjectRecordsOperationType::Update => rule.can_update_records,
                        ObjectRecordsOperationType::SoftDelete => rule.can_soft_delete_records,
                        ObjectRecordsOperationType::Destroy => rule.can_destroy_records,
                    })
                })
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RowLevelRecordsPermissions {
    pub rule_id: String,
    pub can_create_records: bool,
    pub can_read_records: bool,
    pub can_update_records: bool,
    pub can_soft_delete_records: bool,
    pub can_destroy_records: bool,
    pub predicates: Vec<serde_json::Value>,
    pub predicate_groups: Vec<serde_json::Value>,
    pub unreadable_fields: Vec<String>,
    pub uneditable_fields: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoleRecordsPermissions {
    pub can_read_all_records: bool,
    pub can_update_all_records: bool,
    pub can_soft_delete_all_records: bool,
    pub can_destroy_all_records: bool,
    pub objects: BTreeMap<String, ObjectRecordsPermissions>,
    pub can_read_some_records: bool,
}

impl RoleRecordsPermissions {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn object(&self, object_metadata_id: Uuid) -> Option<&ObjectRecordsPermissions> {
        self.objects.get(&object_metadata_id.to_string())
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorkspacePermissionSet {
    pub roles: HashMap<Uuid, RoleRecordsPermissions>,
    pub permission_flags: HashMap<Uuid, HashSet<String>>,
    pub object_ids: BTreeSet<Uuid>,
    pub object_order: Vec<Uuid>,
}

impl WorkspacePermissionSet {
    pub fn role(&self, role_id: Uuid) -> Option<&RoleRecordsPermissions> {
        self.roles.get(&role_id)
    }

    pub fn flags(&self, role_id: Uuid) -> Option<&HashSet<String>> {
        self.permission_flags.get(&role_id)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordReadRestrictions {
    pub unreadable_object_metadata_ids: BTreeSet<Uuid>,
    pub unreadable_field_metadata_ids: BTreeSet<Uuid>,
}

impl RecordReadRestrictions {
    pub fn from_records_permissions(permissions: &RoleRecordsPermissions) -> Self {
        let mut restrictions = Self::default();
        for (object_id, object) in &permissions.objects {
            if !object.can_read_some_records {
                if let Ok(id) = Uuid::parse_str(object_id) {
                    restrictions.unreadable_object_metadata_ids.insert(id);
                }
            }
            for field_id in &object.unreadable_fields {
                if let Ok(id) = Uuid::parse_str(field_id) {
                    restrictions.unreadable_field_metadata_ids.insert(id);
                }
            }
        }
        restrictions
    }
}

pub fn full_access_records_permissions(
    object_ids: impl IntoIterator<Item = Uuid>,
) -> RoleRecordsPermissions {
    let objects = object_ids
        .into_iter()
        .map(|id| {
            (
                id.to_string(),
                ObjectRecordsPermissions {
                    object_metadata_id: id.to_string(),
                    can_create_all_records: true,
                    can_read_all_records: true,
                    can_update_all_records: true,
                    can_soft_delete_all_records: true,
                    can_destroy_all_records: true,
                    unreadable_fields: Vec::new(),
                    uneditable_fields: Vec::new(),
                    row_level_records_permissions: None,
                    can_read_some_records: true,
                },
            )
        })
        .collect();
    RoleRecordsPermissions {
        can_read_all_records: true,
        can_update_all_records: true,
        can_soft_delete_all_records: true,
        can_destroy_all_records: true,
        objects,
        can_read_some_records: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_follows_update_until_the_feature_flag_is_enabled() {
        assert_eq!(
            ObjectRecordsOperationType::Create.enforced(),
            ObjectRecordsOperationType::Update
        );
    }

    #[test]
    fn read_restrictions_include_objects_and_fields() {
        let object_id = Uuid::new_v4();
        let field_id = Uuid::new_v4();
        let permissions = RoleRecordsPermissions {
            objects: BTreeMap::from([(
                object_id.to_string(),
                ObjectRecordsPermissions {
                    unreadable_fields: vec![field_id.to_string()],
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let restrictions = RecordReadRestrictions::from_records_permissions(&permissions);
        assert!(restrictions
            .unreadable_object_metadata_ids
            .contains(&object_id));
        assert!(restrictions
            .unreadable_field_metadata_ids
            .contains(&field_id));
    }
}
