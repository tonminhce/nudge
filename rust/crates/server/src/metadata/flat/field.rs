#![allow(dead_code)]

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use super::enums::FieldMetadataType;
use super::timestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatFieldMetadata {
    #[serde(default)]
    pub id: Uuid,
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_id: Uuid,
    #[serde(default)]
    pub workspace_id: Uuid,
    #[serde(default)]
    pub object_metadata_id: Uuid,
    #[serde(default)]
    pub object_metadata_universal_identifier: String,
    #[serde(rename = "type")]
    pub field_type: FieldMetadataType,
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub is_system_side_effect: bool,
    #[serde(default = "default_true")]
    pub is_nullable: bool,
    #[serde(default)]
    pub is_unique: bool,
    #[serde(default)]
    pub is_searchable: bool,
    #[serde(default = "default_true")]
    pub is_audit_logged: bool,
    #[serde(default = "default_true", rename = "isUIEditable")]
    pub is_ui_editable: bool,
    #[serde(default)]
    pub disabled_field_permission_actions: Option<serde_json::Value>,
    #[serde(default)]
    pub is_label_synced_with_name: bool,
    #[serde(default)]
    pub overrides: Option<serde_json::Value>,
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_target_field_metadata_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_target_object_metadata_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub morph_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_target_field_metadata_universal_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_target_object_metadata_universal_identifier: Option<String>,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub application_universal_identifier: String,
    #[serde(default)]
    pub view_field_ids: Vec<String>,
    #[serde(default)]
    pub view_filter_ids: Vec<String>,
    #[serde(default)]
    pub field_permission_ids: Vec<String>,
    #[serde(default)]
    pub kanban_aggregate_operation_view_ids: Vec<String>,
    #[serde(default)]
    pub calendar_view_ids: Vec<String>,
    #[serde(default)]
    pub calendar_end_view_ids: Vec<String>,
    #[serde(default)]
    pub main_group_by_field_metadata_view_ids: Vec<String>,
    #[serde(default)]
    pub view_sort_ids: Vec<String>,
    #[serde(default)]
    pub search_field_metadata_ids: Vec<String>,
    #[serde(default)]
    pub field_metadata_option_ids: Vec<String>,
    #[serde(default)]
    pub row_level_permission_predicate_ids: Vec<String>,
    #[serde(default)]
    pub relation_row_level_permission_predicate_ids: Vec<String>,
    #[serde(default)]
    pub workspace_member_row_level_permission_predicate_ids: Vec<String>,
    #[serde(default)]
    pub view_filter_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub view_field_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub field_permission_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub kanban_aggregate_operation_view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub calendar_view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub calendar_end_view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub main_group_by_field_metadata_view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub view_sort_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub search_field_metadata_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub field_metadata_option_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub row_level_permission_predicate_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub relation_row_level_permission_predicate_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub workspace_member_row_level_permission_predicate_universal_identifiers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub universal_settings: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl Default for FlatFieldMetadata {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            universal_identifier: String::new(),
            application_id: Uuid::nil(),
            workspace_id: Uuid::nil(),
            object_metadata_id: Uuid::nil(),
            object_metadata_universal_identifier: String::new(),
            field_type: FieldMetadataType::Text,
            name: String::new(),
            label: String::new(),
            description: None,
            icon: None,
            is_active: true,
            is_system: false,
            is_system_side_effect: false,
            is_nullable: true,
            is_unique: false,
            is_searchable: false,
            is_audit_logged: true,
            is_ui_editable: true,
            disabled_field_permission_actions: None,
            is_label_synced_with_name: false,
            overrides: None,
            default_value: None,
            settings: None,
            options: None,
            relation_target_field_metadata_id: None,
            relation_target_object_metadata_id: None,
            morph_id: None,
            relation_target_field_metadata_universal_identifier: None,
            relation_target_object_metadata_universal_identifier: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            application_universal_identifier: String::new(),
            view_field_ids: Vec::new(),
            view_filter_ids: Vec::new(),
            field_permission_ids: Vec::new(),
            kanban_aggregate_operation_view_ids: Vec::new(),
            calendar_view_ids: Vec::new(),
            calendar_end_view_ids: Vec::new(),
            main_group_by_field_metadata_view_ids: Vec::new(),
            view_sort_ids: Vec::new(),
            search_field_metadata_ids: Vec::new(),
            field_metadata_option_ids: Vec::new(),
            row_level_permission_predicate_ids: Vec::new(),
            relation_row_level_permission_predicate_ids: Vec::new(),
            workspace_member_row_level_permission_predicate_ids: Vec::new(),
            view_filter_universal_identifiers: Vec::new(),
            view_field_universal_identifiers: Vec::new(),
            field_permission_universal_identifiers: Vec::new(),
            kanban_aggregate_operation_view_universal_identifiers: Vec::new(),
            calendar_view_universal_identifiers: Vec::new(),
            calendar_end_view_universal_identifiers: Vec::new(),
            main_group_by_field_metadata_view_universal_identifiers: Vec::new(),
            view_sort_universal_identifiers: Vec::new(),
            search_field_metadata_universal_identifiers: Vec::new(),
            field_metadata_option_universal_identifiers: Vec::new(),
            row_level_permission_predicate_universal_identifiers: Vec::new(),
            relation_row_level_permission_predicate_universal_identifiers: Vec::new(),
            workspace_member_row_level_permission_predicate_universal_identifiers: Vec::new(),
            universal_settings: None,
        }
    }
}

impl FlatFieldMetadata {
    pub fn is_relation_or_morph(&self) -> bool {
        self.field_type.is_relation_or_morph()
    }

    pub fn is_enum_type(&self) -> bool {
        self.field_type.is_enum_type()
    }

    pub fn minimal_information() -> FieldMinimalInformation {
        FieldMinimalInformation::default()
    }

    pub fn universal(self) -> UniversalFlatFieldMetadata {
        UniversalFlatFieldMetadata {
            universal_identifier: self.universal_identifier,
            application_universal_identifier: self.application_universal_identifier,
            object_metadata_universal_identifier: self.object_metadata_universal_identifier,
            field_type: self.field_type,
            name: self.name,
            label: self.label,
            description: self.description,
            icon: self.icon,
            is_active: self.is_active,
            is_system: self.is_system,
            is_system_side_effect: self.is_system_side_effect,
            is_nullable: self.is_nullable,
            is_unique: self.is_unique,
            is_searchable: self.is_searchable,
            is_audit_logged: self.is_audit_logged,
            is_ui_editable: self.is_ui_editable,
            disabled_field_permission_actions: self.disabled_field_permission_actions,
            is_label_synced_with_name: self.is_label_synced_with_name,
            overrides: self.overrides,
            default_value: self.default_value,
            universal_settings: self.universal_settings.or(self.settings),
            options: self.options,
            morph_id: self.morph_id,
            relation_target_field_metadata_universal_identifier: self
                .relation_target_field_metadata_universal_identifier,
            relation_target_object_metadata_universal_identifier: self
                .relation_target_object_metadata_universal_identifier,
            created_at: self.created_at,
            updated_at: self.updated_at,
            view_filter_universal_identifiers: self.view_filter_universal_identifiers,
            view_field_universal_identifiers: self.view_field_universal_identifiers,
            field_permission_universal_identifiers: self.field_permission_universal_identifiers,
            kanban_aggregate_operation_view_universal_identifiers: self
                .kanban_aggregate_operation_view_universal_identifiers,
            calendar_view_universal_identifiers: self.calendar_view_universal_identifiers,
            calendar_end_view_universal_identifiers: self.calendar_end_view_universal_identifiers,
            main_group_by_field_metadata_view_universal_identifiers: self
                .main_group_by_field_metadata_view_universal_identifiers,
            view_sort_universal_identifiers: self.view_sort_universal_identifiers,
            search_field_metadata_universal_identifiers: self
                .search_field_metadata_universal_identifiers,
            field_metadata_option_universal_identifiers: self
                .field_metadata_option_universal_identifiers,
            row_level_permission_predicate_universal_identifiers: self
                .row_level_permission_predicate_universal_identifiers,
            relation_row_level_permission_predicate_universal_identifiers: self
                .relation_row_level_permission_predicate_universal_identifiers,
            workspace_member_row_level_permission_predicate_universal_identifiers: self
                .workspace_member_row_level_permission_predicate_universal_identifiers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldMinimalInformation {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_metadata_universal_identifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniversalFlatFieldMetadata {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_universal_identifier: String,
    #[serde(default)]
    pub object_metadata_universal_identifier: String,
    #[serde(rename = "type")]
    pub field_type: FieldMetadataType,
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub is_system_side_effect: bool,
    #[serde(default = "default_true")]
    pub is_nullable: bool,
    #[serde(default)]
    pub is_unique: bool,
    #[serde(default)]
    pub is_searchable: bool,
    #[serde(default = "default_true")]
    pub is_audit_logged: bool,
    #[serde(default = "default_true", rename = "isUIEditable")]
    pub is_ui_editable: bool,
    #[serde(default)]
    pub disabled_field_permission_actions: Option<serde_json::Value>,
    #[serde(default)]
    pub is_label_synced_with_name: bool,
    #[serde(default)]
    pub overrides: Option<serde_json::Value>,
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub universal_settings: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub morph_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_target_field_metadata_universal_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_target_object_metadata_universal_identifier: Option<String>,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub view_filter_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub view_field_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub field_permission_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub kanban_aggregate_operation_view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub calendar_view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub calendar_end_view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub main_group_by_field_metadata_view_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub view_sort_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub search_field_metadata_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub field_metadata_option_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub row_level_permission_predicate_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub relation_row_level_permission_predicate_universal_identifiers: Vec<String>,
    #[serde(default)]
    pub workspace_member_row_level_permission_predicate_universal_identifiers: Vec<String>,
}

impl Default for UniversalFlatFieldMetadata {
    fn default() -> Self {
        FlatFieldMetadata::default().universal()
    }
}

impl UniversalFlatFieldMetadata {
    pub fn is_relation_or_morph(&self) -> bool {
        self.field_type.is_relation_or_morph()
    }

    pub fn is_enum_type(&self) -> bool {
        self.field_type.is_enum_type()
    }

    pub fn is_morph_relation_head(&self) -> bool {
        self.field_type == FieldMetadataType::MorphRelation && self.morph_id.is_some()
    }
}
