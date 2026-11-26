#![allow(dead_code)]

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use super::enums::ObjectOpenRecordIn;
use super::timestamp;

pub const DEPRECATED_TARGET_TABLE_NAME: &str = "DEPRECATED";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatObjectMetadata {
    #[serde(default)]
    pub id: Uuid,
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_id: Uuid,
    #[serde(default)]
    pub workspace_id: Uuid,
    pub name_singular: String,
    pub name_plural: String,
    pub label_singular: String,
    pub label_plural: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub is_remote: bool,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub is_searchable: bool,
    #[serde(default = "default_true")]
    pub is_audit_logged: bool,
    #[serde(default = "default_true", rename = "isUIEditable")]
    pub is_ui_editable: bool,
    #[serde(default = "default_true", rename = "isUICreatable")]
    pub is_ui_creatable: bool,
    #[serde(default)]
    pub disabled_object_permission_actions: Option<serde_json::Value>,
    #[serde(default)]
    pub open_record_in: Option<ObjectOpenRecordIn>,
    #[serde(default)]
    pub is_label_synced_with_name: bool,
    #[serde(default)]
    pub overrides: Option<serde_json::Value>,
    #[serde(default)]
    pub duplicate_criteria: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_identifier_field_metadata_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_identifier_field_metadata_id: Option<Uuid>,
    #[serde(default)]
    pub target_table_name: Option<String>,
    #[serde(default)]
    pub field_ids: Vec<String>,
    #[serde(default)]
    pub index_metadata_ids: Vec<String>,
    #[serde(default)]
    pub search_field_metadata_ids: Vec<String>,
    #[serde(default)]
    pub navigation_menu_item_ids: Vec<String>,
    #[serde(default)]
    pub command_menu_item_ids: Vec<String>,
    #[serde(default)]
    pub object_permission_ids: Vec<String>,
    #[serde(default)]
    pub field_permission_ids: Vec<String>,
    #[serde(default)]
    pub row_level_permission_rule_ids: Vec<String>,
    #[serde(default)]
    pub view_ids: Vec<String>,
    #[serde(default)]
    pub page_layout_ids: Vec<String>,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub application_universal_identifier: String,
    #[serde(default)]
    pub field_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub object_permission_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub field_permission_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub row_level_permission_rule_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub index_metadata_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub search_field_metadata_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub page_layout_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub navigation_menu_item_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub command_menu_item_universal_identifiers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_identifier_field_metadata_universal_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_identifier_field_metadata_universal_identifier: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "isUIReadOnly"
    )]
    pub is_ui_read_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_overrides: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_custom: Option<bool>,
}

fn default_true() -> bool {
    true
}

impl Default for FlatObjectMetadata {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            universal_identifier: String::new(),
            application_id: Uuid::nil(),
            workspace_id: Uuid::nil(),
            name_singular: String::new(),
            name_plural: String::new(),
            label_singular: String::new(),
            label_plural: String::new(),
            color: None,
            description: None,
            icon: None,
            is_remote: false,
            is_active: false,
            is_system: false,
            is_searchable: false,
            is_audit_logged: true,
            is_ui_editable: true,
            is_ui_creatable: true,
            disabled_object_permission_actions: None,
            open_record_in: Some(ObjectOpenRecordIn::UserChoice),
            is_label_synced_with_name: false,
            overrides: None,
            duplicate_criteria: None,
            shortcut: None,
            label_identifier_field_metadata_id: None,
            image_identifier_field_metadata_id: None,
            target_table_name: Some(DEPRECATED_TARGET_TABLE_NAME.to_string()),
            field_ids: Vec::new(),
            index_metadata_ids: Vec::new(),
            search_field_metadata_ids: Vec::new(),
            navigation_menu_item_ids: Vec::new(),
            command_menu_item_ids: Vec::new(),
            object_permission_ids: Vec::new(),
            field_permission_ids: Vec::new(),
            row_level_permission_rule_ids: Vec::new(),
            view_ids: Vec::new(),
            page_layout_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            application_universal_identifier: String::new(),
            field_universal_identifiers: Vec::new(),
            object_permission_universal_identifiers: Vec::new(),
            field_permission_universal_identifiers: Vec::new(),
            row_level_permission_rule_universal_identifiers: Vec::new(),
            view_universal_identifiers: Vec::new(),
            index_metadata_universal_identifiers: Vec::new(),
            search_field_metadata_universal_identifiers: Vec::new(),
            page_layout_universal_identifiers: Vec::new(),
            navigation_menu_item_universal_identifiers: Vec::new(),
            command_menu_item_universal_identifiers: Vec::new(),
            label_identifier_field_metadata_universal_identifier: None,
            image_identifier_field_metadata_universal_identifier: None,
            is_ui_read_only: None,
            standard_overrides: None,
            is_custom: None,
        }
    }
}

impl FlatObjectMetadata {
    pub fn minimal_information(&self) -> ObjectMinimalInformation {
        ObjectMinimalInformation {
            universal_identifier: self.universal_identifier.clone(),
            name_singular: Some(self.name_singular.clone()),
            name_plural: Some(self.name_plural.clone()),
        }
    }

    pub fn universal(self) -> UniversalFlatObjectMetadata {
        UniversalFlatObjectMetadata {
            universal_identifier: self.universal_identifier,
            application_universal_identifier: self.application_universal_identifier,
            name_singular: self.name_singular,
            name_plural: self.name_plural,
            label_singular: self.label_singular,
            label_plural: self.label_plural,
            color: self.color,
            description: self.description,
            icon: self.icon,
            is_remote: self.is_remote,
            is_active: self.is_active,
            is_system: self.is_system,
            is_searchable: self.is_searchable,
            is_audit_logged: self.is_audit_logged,
            is_ui_editable: self.is_ui_editable,
            is_ui_creatable: self.is_ui_creatable,
            disabled_object_permission_actions: self.disabled_object_permission_actions,
            open_record_in: self.open_record_in,
            is_label_synced_with_name: self.is_label_synced_with_name,
            overrides: self.overrides,
            duplicate_criteria: self.duplicate_criteria,
            shortcut: self.shortcut,
            target_table_name: self.target_table_name,
            created_at: self.created_at,
            updated_at: self.updated_at,
            field_universal_identifiers: self.field_universal_identifiers,
            object_permission_universal_identifiers: self.object_permission_universal_identifiers,
            field_permission_universal_identifiers: self.field_permission_universal_identifiers,
            row_level_permission_rule_universal_identifiers: self
                .row_level_permission_rule_universal_identifiers,
            view_universal_identifiers: self.view_universal_identifiers,
            index_metadata_universal_identifiers: self.index_metadata_universal_identifiers,
            search_field_metadata_universal_identifiers: self
                .search_field_metadata_universal_identifiers,
            page_layout_universal_identifiers: self.page_layout_universal_identifiers,
            navigation_menu_item_universal_identifiers: self
                .navigation_menu_item_universal_identifiers,
            command_menu_item_universal_identifiers: self.command_menu_item_universal_identifiers,
            label_identifier_field_metadata_universal_identifier: self
                .label_identifier_field_metadata_universal_identifier,
            image_identifier_field_metadata_universal_identifier: self
                .image_identifier_field_metadata_universal_identifier,
            is_ui_read_only: self.is_ui_read_only,
            is_custom: self.is_custom,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectMinimalInformation {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_singular: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_plural: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniversalFlatObjectMetadata {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_universal_identifier: String,
    pub name_singular: String,
    pub name_plural: String,
    pub label_singular: String,
    pub label_plural: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub is_remote: bool,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub is_searchable: bool,
    #[serde(default = "default_true")]
    pub is_audit_logged: bool,
    #[serde(default = "default_true", rename = "isUIEditable")]
    pub is_ui_editable: bool,
    #[serde(default = "default_true", rename = "isUICreatable")]
    pub is_ui_creatable: bool,
    #[serde(default)]
    pub disabled_object_permission_actions: Option<serde_json::Value>,
    #[serde(default)]
    pub open_record_in: Option<ObjectOpenRecordIn>,
    #[serde(default)]
    pub is_label_synced_with_name: bool,
    #[serde(default)]
    pub overrides: Option<serde_json::Value>,
    #[serde(default)]
    pub duplicate_criteria: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub target_table_name: Option<String>,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub field_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub object_permission_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub field_permission_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub row_level_permission_rule_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub index_metadata_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub search_field_metadata_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub page_layout_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub navigation_menu_item_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub command_menu_item_universal_identifiers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_identifier_field_metadata_universal_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_identifier_field_metadata_universal_identifier: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "isUIReadOnly"
    )]
    pub is_ui_read_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_custom: Option<bool>,
}

impl Default for UniversalFlatObjectMetadata {
    fn default() -> Self {
        FlatObjectMetadata::default().universal()
    }
}

impl UniversalFlatObjectMetadata {
    pub fn minimal_information(&self) -> ObjectMinimalInformation {
        ObjectMinimalInformation {
            universal_identifier: self.universal_identifier.clone(),
            name_singular: Some(self.name_singular.clone()),
            name_plural: Some(self.name_plural.clone()),
        }
    }
}
