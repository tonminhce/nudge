#![allow(dead_code)]

use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;
use sqlx::FromRow;
use uuid::Uuid;

use super::enums::FieldMetadataType;
use super::enums::IndexType;
use super::enums::ObjectOpenRecordIn;
use super::field::FlatFieldMetadata;
use super::index::FlatIndexFieldMetadata;
use super::index::FlatIndexMetadata;
use super::index::UniversalFlatIndexFieldMetadata;
use super::maps::FlatEntityMaps;
use super::maps::FlatWorkspaceMaps;
use super::object::FlatObjectMetadata;
use super::option::FlatFieldMetadataOption;
use super::search_field::FlatSearchFieldMetadata;
use crate::metadata::error::EngineError;
use crate::metadata::error::MetadataError;
use crate::metadata::error::MetadataResult;

#[derive(Debug, Clone, FromRow)]
pub struct ObjectMetadataRow {
    pub id: Uuid,
    #[sqlx(rename = "universalIdentifier")]
    pub universal_identifier: Uuid,
    #[sqlx(rename = "applicationId")]
    pub application_id: Uuid,
    #[sqlx(rename = "workspaceId")]
    pub workspace_id: Uuid,
    #[sqlx(rename = "nameSingular")]
    pub name_singular: String,
    #[sqlx(rename = "namePlural")]
    pub name_plural: String,
    #[sqlx(rename = "labelSingular")]
    pub label_singular: String,
    #[sqlx(rename = "labelPlural")]
    pub label_plural: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    #[sqlx(rename = "targetTableName")]
    pub target_table_name: String,
    #[sqlx(rename = "isRemote")]
    pub is_remote: bool,
    #[sqlx(rename = "isActive")]
    pub is_active: bool,
    #[sqlx(rename = "isSystem")]
    pub is_system: bool,
    #[sqlx(rename = "isUIReadOnly")]
    pub is_ui_read_only: bool,
    #[sqlx(rename = "isUIEditable")]
    pub is_ui_editable: bool,
    #[sqlx(rename = "isUICreatable")]
    pub is_ui_creatable: bool,
    #[sqlx(rename = "isAuditLogged")]
    pub is_audit_logged: bool,
    #[sqlx(rename = "isSearchable")]
    pub is_searchable: bool,
    #[sqlx(rename = "isLabelSyncedWithName")]
    pub is_label_synced_with_name: bool,
    #[sqlx(rename = "openRecordIn")]
    pub open_record_in: String,
    #[sqlx(rename = "disabledObjectPermissionActions")]
    pub disabled_object_permission_actions: Option<serde_json::Value>,
    #[sqlx(rename = "duplicateCriteria")]
    pub duplicate_criteria: Option<serde_json::Value>,
    pub shortcut: Option<String>,
    pub overrides: Option<serde_json::Value>,
    #[sqlx(rename = "labelIdentifierFieldMetadataId")]
    pub label_identifier_field_metadata_id: Option<Uuid>,
    #[sqlx(rename = "imageIdentifierFieldMetadataId")]
    pub image_identifier_field_metadata_id: Option<Uuid>,
    #[sqlx(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct FieldMetadataRow {
    pub id: Uuid,
    #[sqlx(rename = "universalIdentifier")]
    pub universal_identifier: Uuid,
    #[sqlx(rename = "applicationId")]
    pub application_id: Uuid,
    #[sqlx(rename = "workspaceId")]
    pub workspace_id: Uuid,
    #[sqlx(rename = "objectMetadataId")]
    pub object_metadata_id: Uuid,
    #[sqlx(rename = "type")]
    pub field_type: String,
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    #[sqlx(rename = "defaultValue")]
    pub default_value: Option<serde_json::Value>,
    pub options: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
    pub overrides: Option<serde_json::Value>,
    #[sqlx(rename = "isActive")]
    pub is_active: bool,
    #[sqlx(rename = "isSystem")]
    pub is_system: bool,
    #[sqlx(rename = "isSystemSideEffect")]
    pub is_system_side_effect: bool,
    #[sqlx(rename = "isNullable")]
    pub is_nullable: bool,
    #[sqlx(rename = "isUnique")]
    pub is_unique: bool,
    #[sqlx(rename = "isAuditLogged")]
    pub is_audit_logged: bool,
    #[sqlx(rename = "isUIEditable")]
    pub is_ui_editable: bool,
    #[sqlx(rename = "disabledFieldPermissionActions")]
    pub disabled_field_permission_actions: Option<serde_json::Value>,
    #[sqlx(rename = "isLabelSyncedWithName")]
    pub is_label_synced_with_name: bool,
    #[sqlx(rename = "relationTargetFieldMetadataId")]
    pub relation_target_field_metadata_id: Option<Uuid>,
    #[sqlx(rename = "relationTargetObjectMetadataId")]
    pub relation_target_object_metadata_id: Option<Uuid>,
    #[sqlx(rename = "morphId")]
    pub morph_id: Option<Uuid>,
    #[sqlx(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct IndexMetadataRow {
    pub id: Uuid,
    #[sqlx(rename = "universalIdentifier")]
    pub universal_identifier: Uuid,
    #[sqlx(rename = "applicationId")]
    pub application_id: Uuid,
    #[sqlx(rename = "workspaceId")]
    pub workspace_id: Uuid,
    pub name: String,
    #[sqlx(rename = "objectMetadataId")]
    pub object_metadata_id: Uuid,
    #[sqlx(rename = "isUnique")]
    pub is_unique: bool,
    #[sqlx(rename = "indexWhereClause")]
    pub index_where_clause: Option<String>,
    #[sqlx(rename = "indexType")]
    pub index_type: String,
    #[sqlx(rename = "isSystemSideEffect")]
    pub is_system_side_effect: bool,
    #[sqlx(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct IndexFieldMetadataRow {
    pub id: Uuid,
    #[sqlx(rename = "indexMetadataId")]
    pub index_metadata_id: Uuid,
    #[sqlx(rename = "fieldMetadataId")]
    pub field_metadata_id: Uuid,
    #[sqlx(rename = "order")]
    pub order: i32,
    #[sqlx(rename = "subFieldName")]
    pub sub_field_name: Option<String>,
    #[sqlx(rename = "workspaceId")]
    pub workspace_id: Uuid,
    #[sqlx(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct FieldMetadataOptionRow {
    pub id: Uuid,
    #[sqlx(rename = "universalIdentifier")]
    pub universal_identifier: Uuid,
    #[sqlx(rename = "applicationId")]
    pub application_id: Uuid,
    #[sqlx(rename = "workspaceId")]
    pub workspace_id: Uuid,
    #[sqlx(rename = "fieldMetadataId")]
    pub field_metadata_id: Uuid,
    pub value: String,
    pub label: String,
    pub color: Option<String>,
    pub position: f64,
    #[sqlx(rename = "isActive")]
    pub is_active: bool,
    pub overrides: Option<serde_json::Value>,
    #[sqlx(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SearchFieldMetadataRow {
    pub id: Uuid,
    #[sqlx(rename = "universalIdentifier")]
    pub universal_identifier: Uuid,
    #[sqlx(rename = "applicationId")]
    pub application_id: Uuid,
    #[sqlx(rename = "workspaceId")]
    pub workspace_id: Uuid,
    #[sqlx(rename = "objectMetadataId")]
    pub object_metadata_id: Uuid,
    #[sqlx(rename = "fieldMetadataId")]
    pub field_metadata_id: Uuid,
    #[sqlx(rename = "tsVectorFieldMetadataId")]
    pub ts_vector_field_metadata_id: Uuid,
    pub position: f64,
    #[sqlx(rename = "isSystemSideEffect")]
    pub is_system_side_effect: bool,
    #[sqlx(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceMetadataRows {
    pub object_metadata: Vec<ObjectMetadataRow>,
    pub field_metadata: Vec<FieldMetadataRow>,
    pub index_metadata: Vec<IndexMetadataRow>,
    pub index_field_metadata: Vec<IndexFieldMetadataRow>,
    pub field_metadata_option: Vec<FieldMetadataOptionRow>,
    pub search_field_metadata: Vec<SearchFieldMetadataRow>,
}

pub fn assemble_workspace_maps(rows: WorkspaceMetadataRows) -> MetadataResult<FlatWorkspaceMaps> {
    let object_universal_by_id: HashMap<Uuid, Uuid> = rows
        .object_metadata
        .iter()
        .map(|row| (row.id, row.universal_identifier))
        .collect();

    let field_universal_by_id: HashMap<Uuid, Uuid> = rows
        .field_metadata
        .iter()
        .map(|row| (row.id, row.universal_identifier))
        .collect();

    let mut object_maps: FlatEntityMaps<FlatObjectMetadata> = FlatEntityMaps::default();

    for row in &rows.object_metadata {
        let flat = FlatObjectMetadata {
            id: row.id,
            universal_identifier: row.universal_identifier.to_string(),
            application_id: row.application_id,
            workspace_id: row.workspace_id,
            name_singular: row.name_singular.clone(),
            name_plural: row.name_plural.clone(),
            label_singular: row.label_singular.clone(),
            label_plural: row.label_plural.clone(),
            color: row.color.clone(),
            description: row.description.clone(),
            icon: row.icon.clone(),
            is_remote: row.is_remote,
            is_active: row.is_active,
            is_system: row.is_system,
            is_searchable: row.is_searchable,
            is_audit_logged: row.is_audit_logged,
            is_ui_editable: row.is_ui_editable,
            is_ui_creatable: row.is_ui_creatable,
            disabled_object_permission_actions: row.disabled_object_permission_actions.clone(),
            open_record_in: parse_open_record_in(&row.open_record_in),
            is_label_synced_with_name: row.is_label_synced_with_name,
            overrides: row.overrides.clone(),
            duplicate_criteria: row.duplicate_criteria.clone(),
            shortcut: row.shortcut.clone(),
            label_identifier_field_metadata_id: row.label_identifier_field_metadata_id,
            image_identifier_field_metadata_id: row.image_identifier_field_metadata_id,
            target_table_name: Some(row.target_table_name.clone()),
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
            created_at: row.created_at,
            updated_at: row.updated_at,
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
            label_identifier_field_metadata_universal_identifier: row
                .label_identifier_field_metadata_id
                .and_then(|field_id| field_universal_by_id.get(&field_id))
                .map(|universal| universal.to_string()),
            image_identifier_field_metadata_universal_identifier: row
                .image_identifier_field_metadata_id
                .and_then(|field_id| field_universal_by_id.get(&field_id))
                .map(|universal| universal.to_string()),
            is_ui_read_only: Some(row.is_ui_read_only),
            standard_overrides: None,
            is_custom: None,
        };

        object_maps.add(
            row.id.to_string(),
            flat,
            row.universal_identifier.to_string(),
            row.application_id.to_string(),
        );
    }

    let mut field_maps: FlatEntityMaps<FlatFieldMetadata> = FlatEntityMaps::default();

    for row in &rows.field_metadata {
        let field_type = FieldMetadataType::parse(&row.field_type).ok_or_else(|| {
            EngineError::internal(
                "BUILDER_INTERNAL_SERVER_ERROR",
                format!(
                    "unknown fieldMetadata type \"{}\" on field {}",
                    row.field_type, row.id
                ),
            )
        })?;

        let flat = FlatFieldMetadata {
            id: row.id,
            universal_identifier: row.universal_identifier.to_string(),
            application_id: row.application_id,
            workspace_id: row.workspace_id,
            object_metadata_id: row.object_metadata_id,
            object_metadata_universal_identifier: resolve_required(
                object_universal_by_id.get(&row.object_metadata_id),
                "fieldMetadata",
                "objectMetadataUniversalIdentifier",
                row.id,
            )?
            .to_string(),
            field_type,
            name: row.name.clone(),
            label: row.label.clone(),
            description: row.description.clone(),
            icon: row.icon.clone(),
            is_active: row.is_active,
            is_system: row.is_system,
            is_system_side_effect: row.is_system_side_effect,
            is_nullable: row.is_nullable,
            is_unique: row.is_unique,
            is_searchable: false,
            is_audit_logged: row.is_audit_logged,
            is_ui_editable: row.is_ui_editable,
            disabled_field_permission_actions: row.disabled_field_permission_actions.clone(),
            is_label_synced_with_name: row.is_label_synced_with_name,
            overrides: row.overrides.clone(),
            default_value: row.default_value.clone(),
            settings: row.settings.clone(),
            options: row.options.clone(),
            relation_target_field_metadata_id: row.relation_target_field_metadata_id,
            relation_target_object_metadata_id: row.relation_target_object_metadata_id,
            morph_id: row.morph_id,
            relation_target_field_metadata_universal_identifier: row
                .relation_target_field_metadata_id
                .and_then(|field_id| field_universal_by_id.get(&field_id))
                .map(|universal| universal.to_string()),
            relation_target_object_metadata_universal_identifier: row
                .relation_target_object_metadata_id
                .and_then(|object_id| object_universal_by_id.get(&object_id))
                .map(|universal| universal.to_string()),
            created_at: row.created_at,
            updated_at: row.updated_at,
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
            universal_settings: row.settings.clone(),
        };

        field_maps.add(
            row.id.to_string(),
            flat,
            row.universal_identifier.to_string(),
            row.application_id.to_string(),
        );
    }

    let mut option_maps: FlatEntityMaps<FlatFieldMetadataOption> = FlatEntityMaps::default();

    for row in &rows.field_metadata_option {
        let field_universal = resolve_required(
            field_universal_by_id.get(&row.field_metadata_id),
            "fieldMetadataOption",
            "fieldMetadataUniversalIdentifier",
            row.id,
        )?;

        let flat = FlatFieldMetadataOption {
            id: row.id,
            universal_identifier: row.universal_identifier.to_string(),
            application_id: row.application_id,
            workspace_id: row.workspace_id,
            field_metadata_id: row.field_metadata_id,
            field_metadata_universal_identifier: field_universal.to_string(),
            value: row.value.clone(),
            label: row.label.clone(),
            color: row.color.clone(),
            description: None,
            position: row.position,
            is_active: row.is_active,
            overrides: row.overrides.clone(),
            created_at: row.created_at,
            updated_at: row.updated_at,
            application_universal_identifier: String::new(),
        };

        option_maps.add(
            row.id.to_string(),
            flat,
            row.universal_identifier.to_string(),
            row.application_id.to_string(),
        );
    }

    let mut index_field_rows_by_index: HashMap<Uuid, Vec<&IndexFieldMetadataRow>> = HashMap::new();

    for row in &rows.index_field_metadata {
        index_field_rows_by_index
            .entry(row.index_metadata_id)
            .or_default()
            .push(row);
    }

    for fields in index_field_rows_by_index.values_mut() {
        fields.sort_by_key(|row| row.order);
    }

    let mut index_maps: FlatEntityMaps<FlatIndexMetadata> = FlatEntityMaps::default();

    for row in &rows.index_metadata {
        let object_universal = resolve_required(
            object_universal_by_id.get(&row.object_metadata_id),
            "index",
            "objectMetadataUniversalIdentifier",
            row.id,
        )?;

        let index_fields = index_field_rows_by_index
            .remove(&row.id)
            .unwrap_or_default();
        let mut flat_index_fields = Vec::with_capacity(index_fields.len());
        let mut universal_index_fields = Vec::with_capacity(index_fields.len());

        for index_field in index_fields {
            let field_universal = resolve_required(
                field_universal_by_id.get(&index_field.field_metadata_id),
                "indexFieldMetadata",
                "fieldMetadataUniversalIdentifier",
                index_field.id,
            )?;

            flat_index_fields.push(FlatIndexFieldMetadata {
                id: index_field.id,
                workspace_id: index_field.workspace_id,
                index_metadata_id: index_field.index_metadata_id,
                field_metadata_id: index_field.field_metadata_id,
                order: index_field.order,
                sub_field_name: index_field.sub_field_name.clone(),
                created_at: index_field.created_at,
                updated_at: index_field.updated_at,
            });

            universal_index_fields.push(UniversalFlatIndexFieldMetadata {
                order: index_field.order,
                sub_field_name: index_field.sub_field_name.clone(),
                created_at: index_field.created_at,
                updated_at: index_field.updated_at,
                index_metadata_universal_identifier: row.universal_identifier.to_string(),
                field_metadata_universal_identifier: field_universal.to_string(),
            });
        }

        let flat = FlatIndexMetadata {
            id: row.id,
            universal_identifier: row.universal_identifier.to_string(),
            application_id: row.application_id,
            workspace_id: row.workspace_id,
            name: row.name.clone(),
            object_metadata_id: row.object_metadata_id,
            object_metadata_universal_identifier: object_universal.to_string(),
            is_unique: row.is_unique,
            index_where_clause: row.index_where_clause.clone(),
            index_type: IndexType::parse(&row.index_type).ok_or_else(|| {
                EngineError::internal(
                    "BUILDER_INTERNAL_SERVER_ERROR",
                    format!(
                        "unknown indexType \"{}\" on index {}",
                        row.index_type, row.id
                    ),
                )
            })?,
            is_system_side_effect: row.is_system_side_effect,
            created_at: row.created_at,
            updated_at: row.updated_at,
            flat_index_field_metadatas: flat_index_fields,
            universal_flat_index_field_metadatas: universal_index_fields,
        };

        index_maps.add(
            row.id.to_string(),
            flat,
            row.universal_identifier.to_string(),
            row.application_id.to_string(),
        );
    }

    let mut search_field_maps: FlatEntityMaps<FlatSearchFieldMetadata> = FlatEntityMaps::default();

    for row in &rows.search_field_metadata {
        let object_universal = resolve_required(
            object_universal_by_id.get(&row.object_metadata_id),
            "searchFieldMetadata",
            "objectMetadataUniversalIdentifier",
            row.id,
        )?;
        let field_universal = resolve_required(
            field_universal_by_id.get(&row.field_metadata_id),
            "searchFieldMetadata",
            "fieldMetadataUniversalIdentifier",
            row.id,
        )?;
        let ts_vector_universal = resolve_required(
            field_universal_by_id.get(&row.ts_vector_field_metadata_id),
            "searchFieldMetadata",
            "tsVectorFieldMetadataUniversalIdentifier",
            row.id,
        )?;

        let flat = FlatSearchFieldMetadata {
            id: row.id,
            universal_identifier: row.universal_identifier.to_string(),
            application_id: row.application_id,
            workspace_id: row.workspace_id,
            object_metadata_id: row.object_metadata_id,
            object_metadata_universal_identifier: object_universal.to_string(),
            field_metadata_id: row.field_metadata_id,
            field_metadata_universal_identifier: field_universal.to_string(),
            ts_vector_field_metadata_id: row.ts_vector_field_metadata_id,
            ts_vector_field_metadata_universal_identifier: ts_vector_universal.to_string(),
            position: row.position,
            is_system_side_effect: row.is_system_side_effect,
            created_at: row.created_at,
            updated_at: row.updated_at,
            application_universal_identifier: String::new(),
        };

        search_field_maps.add(
            row.id.to_string(),
            flat,
            row.universal_identifier.to_string(),
            row.application_id.to_string(),
        );
    }

    let mut maps = FlatWorkspaceMaps {
        object_metadata_maps: object_maps,
        field_metadata_maps: field_maps,
        index_metadata_maps: index_maps,
        field_metadata_option_maps: option_maps,
        search_field_metadata_maps: search_field_maps,
    };

    backfill_child_identifiers(&mut maps, &field_universal_by_id);

    Ok(maps)
}

fn backfill_child_identifiers(
    maps: &mut FlatWorkspaceMaps,
    field_universal_by_id: &HashMap<Uuid, Uuid>,
) {
    let mut field_ids_by_object: HashMap<Uuid, (Vec<String>, Vec<String>)> = HashMap::new();

    for field in maps.field_metadata_maps.by_universal_identifier.values() {
        let entry = field_ids_by_object
            .entry(field.object_metadata_id)
            .or_default();

        entry.0.push(field.id.to_string());
        entry.1.push(field.universal_identifier.clone());
    }

    let mut index_ids_by_object: HashMap<Uuid, (Vec<String>, Vec<String>)> = HashMap::new();

    for index in maps.index_metadata_maps.by_universal_identifier.values() {
        let entry = index_ids_by_object
            .entry(index.object_metadata_id)
            .or_default();

        entry.0.push(index.id.to_string());
        entry.1.push(index.universal_identifier.clone());
    }

    let mut search_field_ids_by_object: HashMap<Uuid, (Vec<String>, Vec<String>)> = HashMap::new();

    for search_field in maps
        .search_field_metadata_maps
        .by_universal_identifier
        .values()
    {
        let entry = search_field_ids_by_object
            .entry(search_field.object_metadata_id)
            .or_default();

        entry.0.push(search_field.id.to_string());
        entry.1.push(search_field.universal_identifier.clone());
    }

    let mut search_field_ids_by_field: HashMap<Uuid, (Vec<String>, Vec<String>)> = HashMap::new();

    for search_field in maps
        .search_field_metadata_maps
        .by_universal_identifier
        .values()
    {
        let entry = search_field_ids_by_field
            .entry(search_field.field_metadata_id)
            .or_default();

        entry.0.push(search_field.id.to_string());
        entry.1.push(search_field.universal_identifier.clone());
    }

    let mut option_ids_by_field: HashMap<Uuid, (Vec<String>, Vec<String>)> = HashMap::new();

    for option in maps
        .field_metadata_option_maps
        .by_universal_identifier
        .values()
    {
        let entry = option_ids_by_field
            .entry(option.field_metadata_id)
            .or_default();

        entry.0.push(option.id.to_string());
        entry.1.push(option.universal_identifier.clone());
    }

    for object in maps
        .object_metadata_maps
        .by_universal_identifier
        .values_mut()
    {
        if let Some((field_ids, field_universal_ids)) = field_ids_by_object.remove(&object.id) {
            object.field_ids = field_ids;
            object.field_universal_identifiers = field_universal_ids;
        }

        if let Some((index_ids, index_universal_ids)) = index_ids_by_object.remove(&object.id) {
            object.index_metadata_ids = index_ids;
            object.index_metadata_universal_identifiers = index_universal_ids;
        }

        if let Some((search_field_ids, search_field_universal_ids)) =
            search_field_ids_by_object.remove(&object.id)
        {
            object.search_field_metadata_ids = search_field_ids;
            object.search_field_metadata_universal_identifiers = search_field_universal_ids;
        }
    }

    for field in maps
        .field_metadata_maps
        .by_universal_identifier
        .values_mut()
    {
        if let Some((search_field_ids, search_field_universal_ids)) =
            search_field_ids_by_field.remove(&field.id)
        {
            field.search_field_metadata_ids = search_field_ids;
            field.search_field_metadata_universal_identifiers = search_field_universal_ids;
        }

        if let Some((option_ids, option_universal_ids)) = option_ids_by_field.remove(&field.id) {
            field.field_metadata_option_ids = option_ids;
            field.field_metadata_option_universal_identifiers = option_universal_ids;
        }
    }

    let _ = field_universal_by_id;
}

fn resolve_required(
    value: Option<&Uuid>,
    metadata_name: &str,
    property: &str,
    id: Uuid,
) -> MetadataResult<Uuid> {
    value.copied().ok_or_else(|| {
        MetadataError::Engine(EngineError::not_found(
            "RELATION_UNIVERSAL_IDENTIFIER_NOT_FOUND",
            format!(
                "could not resolve {property} of {metadata_name} row {id}: related entity not found in the loaded maps"
            ),
        ))
    })
}

fn parse_open_record_in(raw: &str) -> Option<ObjectOpenRecordIn> {
    match raw {
        "SIDE_PANEL" => Some(ObjectOpenRecordIn::SidePanel),
        "RECORD_PAGE" => Some(ObjectOpenRecordIn::RecordPage),
        "USER_CHOICE" => Some(ObjectOpenRecordIn::UserChoice),
        _ => None,
    }
}

pub async fn load_workspace_flat_maps(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> MetadataResult<FlatWorkspaceMaps> {
    let object_rows = load_object_metadata_rows(pool, workspace_id).await?;
    let field_rows = load_field_metadata_rows(pool, workspace_id).await?;
    let index_rows = load_index_metadata_rows(pool, workspace_id).await?;
    let index_field_rows = load_index_field_metadata_rows(pool, workspace_id).await?;
    let option_rows = load_field_metadata_option_rows(pool, workspace_id).await?;
    let search_field_rows = load_search_field_metadata_rows(pool, workspace_id).await?;

    assemble_workspace_maps(WorkspaceMetadataRows {
        object_metadata: object_rows,
        field_metadata: field_rows,
        index_metadata: index_rows,
        index_field_metadata: index_field_rows,
        field_metadata_option: option_rows,
        search_field_metadata: search_field_rows,
    })
}

async fn load_object_metadata_rows(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> MetadataResult<Vec<ObjectMetadataRow>> {
    let rows = sqlx::query_as::<_, ObjectMetadataRow>(
        "SELECT \"id\", \"universalIdentifier\", \"applicationId\", \"workspaceId\", \
         \"nameSingular\", \"namePlural\", \"labelSingular\", \"labelPlural\", \"description\", \
         \"icon\", \"color\", \"targetTableName\", \"isRemote\", \"isActive\", \"isSystem\", \
         \"isUIReadOnly\", \"isUIEditable\", \"isUICreatable\", \"isAuditLogged\", \"isSearchable\", \
         \"isLabelSyncedWithName\", \"openRecordIn\"::text AS \"openRecordIn\", \
         \"disabledObjectPermissionActions\", \"duplicateCriteria\", \"shortcut\", \"overrides\", \
         \"labelIdentifierFieldMetadataId\", \"imageIdentifierFieldMetadataId\", \
         \"createdAt\", \"updatedAt\" \
         FROM core.\"objectMetadata\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

async fn load_field_metadata_rows(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> MetadataResult<Vec<FieldMetadataRow>> {
    let rows = sqlx::query_as::<_, FieldMetadataRow>(
        "SELECT \"id\", \"universalIdentifier\", \"applicationId\", \"workspaceId\", \
         \"objectMetadataId\", \"type\"::text AS \"type\", \"name\", \"label\", \"description\", \
         \"icon\", \"defaultValue\", \"options\", \"settings\", \"overrides\", \"isActive\", \
         \"isSystem\", \"isSystemSideEffect\", \"isNullable\", \"isUnique\", \"isAuditLogged\", \
         \"isUIEditable\", \"disabledFieldPermissionActions\", \"isLabelSyncedWithName\", \
         \"relationTargetFieldMetadataId\", \"relationTargetObjectMetadataId\", \"morphId\", \
         \"createdAt\", \"updatedAt\" \
         FROM core.\"fieldMetadata\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

async fn load_index_metadata_rows(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> MetadataResult<Vec<IndexMetadataRow>> {
    let rows = sqlx::query_as::<_, IndexMetadataRow>(
        "SELECT \"id\", \"universalIdentifier\", \"applicationId\", \"workspaceId\", \"name\", \
         \"objectMetadataId\", \"isUnique\", \"indexWhereClause\", \"indexType\"::text AS \
         \"indexType\", \"isSystemSideEffect\", \"createdAt\", \"updatedAt\" \
         FROM core.\"indexMetadata\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

async fn load_index_field_metadata_rows(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> MetadataResult<Vec<IndexFieldMetadataRow>> {
    let rows = sqlx::query_as::<_, IndexFieldMetadataRow>(
        "SELECT \"id\", \"indexMetadataId\", \"fieldMetadataId\", \"order\", \"subFieldName\", \
         \"workspaceId\", \"createdAt\", \"updatedAt\" \
         FROM core.\"indexFieldMetadata\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

async fn load_field_metadata_option_rows(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> MetadataResult<Vec<FieldMetadataOptionRow>> {
    let rows = sqlx::query_as::<_, FieldMetadataOptionRow>(
        "SELECT \"id\", \"universalIdentifier\", \"applicationId\", \"workspaceId\", \
         \"fieldMetadataId\", \"value\", \"label\", \"color\", \"position\", \"isActive\", \
         \"overrides\", \"createdAt\", \"updatedAt\" \
         FROM core.\"fieldMetadataOption\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

async fn load_search_field_metadata_rows(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> MetadataResult<Vec<SearchFieldMetadataRow>> {
    let rows = sqlx::query_as::<_, SearchFieldMetadataRow>(
        "SELECT \"id\", \"universalIdentifier\", \"applicationId\", \"workspaceId\", \
         \"objectMetadataId\", \"fieldMetadataId\", \"tsVectorFieldMetadataId\", \"position\", \
         \"isSystemSideEffect\", \"createdAt\", \"updatedAt\" \
         FROM core.\"searchFieldMetadata\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
