use chrono::SecondsFormat;
use chrono::Utc;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use super::errors::GraphQlError;
use super::graphql::FieldSelection;
use super::input::compute_deterministic_uuid;
use super::input::nullify_empty_composite_default_value;
use super::input::optional_bool;
use super::input::optional_string;
use super::input::rating_options;
use super::input::require_string;
use super::input::require_uuid;
use super::input::sanitize_select_options;
use super::input::sanitize_string_fields;
use crate::metadata::actions::comparable_properties;
use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalFlatEntity;
use crate::metadata::builders::BuildOptions;
use crate::metadata::builders::FlatEntityOperationsMatrix;
use crate::metadata::builders::InferDeletionFromMissingEntities;
use crate::metadata::builders::OrchestratorFailureReport;
use crate::metadata::cache::to_universal_workspace_maps;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FlatEntityMaps;
use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::FlatFieldMetadataOption;
use crate::metadata::flat::FlatIndexMetadata;
use crate::metadata::flat::FlatObjectMetadata;
use crate::metadata::flat::FlatWorkspaceMaps;
use crate::metadata::flat::FromToAllMaps;
use crate::metadata::flat::FromToMaps;
use crate::metadata::flat::IndexType;
use crate::metadata::flat::ObjectOpenRecordIn;
use crate::metadata::flat::UniversalFlatFieldMetadata;
use crate::metadata::flat::UniversalFlatFieldMetadataOption;
use crate::metadata::flat::UniversalFlatIndexFieldMetadata;
use crate::metadata::flat::UniversalFlatIndexMetadata;
use crate::metadata::flat::UniversalFlatObjectMetadata;
use crate::metadata::flat::UniversalWorkspaceMaps;
use crate::metadata::ids::generate_deterministic_index_name;
use crate::metadata::names::capitalize;
use crate::metadata::names::compute_composite_column_name;
use crate::metadata::names::compute_metadata_name_from_label;
use crate::metadata::names::compute_object_target_table;
use crate::metadata::names::compute_relation_join_column_name;
use crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER;
use crate::state::AppState;

pub const FIELD_METADATA_NAMESPACE: &str = "fieldMetadata";
pub const MAX_OBJECTS_PER_BATCH_UPDATE: usize = 100;
pub const MAX_CUSTOM_INDEXES_PER_OBJECT: usize = 10;
pub const DEPRECATED_TARGET_TABLE_NAME: &str = "DEPRECATED";
pub const NON_BATCHABLE_UPDATE_PROPERTIES: &[&str] = &[
    "nameSingular",
    "namePlural",
    "labelIdentifierFieldMetadataId",
];
pub const GIN_COMPATIBLE_FIELD_TYPES: &[&str] = &["TS_VECTOR", "ARRAY", "MULTI_SELECT", "RAW_JSON"];

pub struct OperationContext<'a> {
    pub state: &'a AppState,
    pub workspace_id: Uuid,
    pub auth: Option<&'a crate::auth::middleware::AuthContext>,
    pub selection: &'a FieldSelection,
    pub variables: &'a Value,
    pub is_query: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedApplications {
    pub custom_universal_identifier: String,
    pub standard_universal_identifier: String,
    pub custom_id: Uuid,
}

pub async fn resolve_applications(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<ResolvedApplications, GraphQlError> {
    let custom: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT a.id, a.\"universalIdentifier\" FROM core.\"workspace\" w \
         JOIN core.\"application\" a ON a.id = w.\"workspaceCustomApplicationId\" \
         WHERE w.id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| GraphQlError::internal(error.to_string()))?;

    let (custom_id, custom_universal_identifier) = custom
        .map(|(id, universal_identifier)| {
            (
                id,
                universal_identifier
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| id.to_string()),
            )
        })
        .ok_or_else(|| GraphQlError::internal("Could not find workspace custom application"))?;

    let has_standard: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"application\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(workspace_id)
    .bind(
        Uuid::parse_str(TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER)
            .expect("standard application universal identifier is a uuid"),
    )
    .fetch_optional(&state.core_pool)
    .await
    .map_err(|error| GraphQlError::internal(error.to_string()))?;

    let standard_universal_identifier = if has_standard.is_some() {
        TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER.to_string()
    } else {
        custom_universal_identifier.clone()
    };

    Ok(ResolvedApplications {
        custom_id,
        custom_universal_identifier,
        standard_universal_identifier,
    })
}

pub async fn load_flat_maps(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<FlatWorkspaceMaps, GraphQlError> {
    state
        .metadata_cache
        .get_or_recompute(&state.core_pool, workspace_id)
        .await
        .map_err(|error| GraphQlError::internal(error.to_string()))
}

pub async fn load_universal_maps(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<UniversalWorkspaceMaps, GraphQlError> {
    Ok(to_universal_workspace_maps(
        &load_flat_maps(state, workspace_id).await?,
    ))
}

pub fn build_options(
    applications: &ResolvedApplications,
    application_universal_identifier: &str,
    infer_deletion: InferDeletionFromMissingEntities,
) -> BuildOptions {
    BuildOptions {
        is_system_build: false,
        application_universal_identifier: application_universal_identifier.to_string(),
        workspace_custom_application_universal_identifier: applications
            .custom_universal_identifier
            .clone(),
        application_default_role_universal_identifier: None,
        infer_deletion_from_missing_entities: infer_deletion,
    }
}

pub fn infer_deletion_for_deletes(
    matrix: &FlatEntityOperationsMatrix,
) -> InferDeletionFromMissingEntities {
    let mut per_name = std::collections::HashMap::new();

    for (metadata_name, operations) in &matrix.by_metadata_name {
        if !operations.to_delete.is_empty() {
            per_name.insert(*metadata_name, true);
        }
    }

    if per_name.is_empty() {
        InferDeletionFromMissingEntities::All(false)
    } else {
        InferDeletionFromMissingEntities::PerName(per_name)
    }
}

pub fn compute_api_from_to_maps(
    maps: &UniversalWorkspaceMaps,
    matrix: &FlatEntityOperationsMatrix,
) -> FromToAllMaps {
    let mut from_to = FromToAllMaps {
        object_metadata: FromToMaps {
            from: maps.object_metadata_maps.clone(),
            to: maps.object_metadata_maps.clone(),
        },
        field_metadata: FromToMaps {
            from: maps.field_metadata_maps.clone(),
            to: maps.field_metadata_maps.clone(),
        },
        index_metadata: FromToMaps {
            from: maps.index_metadata_maps.clone(),
            to: maps.index_metadata_maps.clone(),
        },
        field_metadata_option: FromToMaps {
            from: maps.field_metadata_option_maps.clone(),
            to: maps.field_metadata_option_maps.clone(),
        },
        search_field_metadata: FromToMaps {
            from: maps.search_field_metadata_maps.clone(),
            to: maps.search_field_metadata_maps.clone(),
        },
    };

    for (metadata_name, operations) in &matrix.by_metadata_name {
        for entity in &operations.to_delete {
            remove_entity_from_bucket(&mut from_to, *metadata_name, entity);
        }

        for entity in operations
            .to_create
            .iter()
            .chain(operations.to_update.iter())
        {
            apply_entity_to_bucket(&mut from_to, *metadata_name, entity);
        }
    }

    from_to
}

fn remove_entity_from_bucket(
    from_to: &mut FromToAllMaps,
    metadata_name: MetadataName,
    entity: &UniversalFlatEntity,
) {
    let identifier = entity.universal_identifier();

    match metadata_name {
        MetadataName::ObjectMetadata => {
            from_to
                .object_metadata
                .to
                .delete_by_universal_identifier(identifier);
        }
        MetadataName::FieldMetadata => {
            from_to
                .field_metadata
                .to
                .delete_by_universal_identifier(identifier);
        }
        MetadataName::Index => {
            from_to
                .index_metadata
                .to
                .delete_by_universal_identifier(identifier);
        }
        MetadataName::FieldMetadataOption => {
            from_to
                .field_metadata_option
                .to
                .delete_by_universal_identifier(identifier);
        }
        MetadataName::SearchFieldMetadata => {
            from_to
                .search_field_metadata
                .to
                .delete_by_universal_identifier(identifier);
        }
        _ => {}
    }
}

fn apply_entity_to_bucket(
    from_to: &mut FromToAllMaps,
    metadata_name: MetadataName,
    entity: &UniversalFlatEntity,
) {
    let Ok(value) = entity.to_value() else {
        return;
    };

    match metadata_name {
        MetadataName::ObjectMetadata => {
            if let Ok(typed) = serde_json::from_value::<UniversalFlatObjectMetadata>(value) {
                from_to
                    .object_metadata
                    .to
                    .add(entity.universal_identifier().to_string(), typed);
            }
        }
        MetadataName::FieldMetadata => {
            if let Ok(typed) = serde_json::from_value::<UniversalFlatFieldMetadata>(value) {
                from_to
                    .field_metadata
                    .to
                    .add(entity.universal_identifier().to_string(), typed);
            }
        }
        MetadataName::Index => {
            if let Ok(typed) = serde_json::from_value::<UniversalFlatIndexMetadata>(value) {
                from_to
                    .index_metadata
                    .to
                    .add(entity.universal_identifier().to_string(), typed);
            }
        }
        MetadataName::FieldMetadataOption => {
            if let Ok(typed) = serde_json::from_value::<UniversalFlatFieldMetadataOption>(value) {
                from_to
                    .field_metadata_option
                    .to
                    .add(entity.universal_identifier().to_string(), typed);
            }
        }
        MetadataName::SearchFieldMetadata => {
            if let Ok(typed) = serde_json::from_value::<
                crate::metadata::flat::UniversalFlatSearchFieldMetadata,
            >(value)
            {
                from_to
                    .search_field_metadata
                    .to
                    .add(entity.universal_identifier().to_string(), typed);
            }
        }
        _ => {}
    }
}

pub fn field_universal_identifier(
    application_universal_identifier: &str,
    object_universal_identifier: &str,
    name: &str,
) -> String {
    compute_deterministic_uuid(
        FIELD_METADATA_NAMESPACE,
        &format!("{object_universal_identifier}:{name}"),
        application_universal_identifier,
    )
}

pub fn valid_metadata_name_shape(name: &str) -> bool {
    !matches!(
        name,
        "not" | "or" | "and" | "Int" | "Float" | "Boolean" | "String" | "ID"
    ) && !name.contains(['\'', '"', '\\', ';', '.', '=', '*', '/'])
}

pub fn validate_create_object_input(input: &Value) -> Result<(), GraphQlError> {
    let object = input
        .get("object")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GraphQlError::validation("Field \"object\" of type \"CreateObjectInput!\" is required.")
        })?;

    let name_singular = require_string(object, "nameSingular")?;
    let name_plural = require_string(object, "namePlural")?;
    let label_singular = require_string(object, "labelSingular")?;
    let label_plural = require_string(object, "labelPlural")?;

    for (field, value) in [
        ("nameSingular", &name_singular),
        ("namePlural", &name_plural),
    ] {
        if !valid_metadata_name_shape(value) {
            return Err(GraphQlError::user_input(format!(
                "{field} has failed the name validation check"
            )));
        }
    }

    for (field, value) in [
        ("nameSingular", &name_singular),
        ("namePlural", &name_plural),
        ("labelSingular", &label_singular),
        ("labelPlural", &label_plural),
    ] {
        if value.is_empty() {
            return Err(GraphQlError::user_input(format!(
                "{field} should not be empty"
            )));
        }
    }

    Ok(())
}

pub fn validate_create_field_input(input: &Value) -> Result<(), GraphQlError> {
    let field = input
        .get("field")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GraphQlError::validation("Field \"field\" of type \"CreateFieldInput!\" is required.")
        })?;

    let name = require_string(field, "name")?;

    if !valid_metadata_name_shape(&name) {
        return Err(GraphQlError::user_input(
            "name has failed the name validation check",
        ));
    }

    if name.is_empty() {
        return Err(GraphQlError::user_input("name should not be empty"));
    }

    let type_raw = require_string(field, "type")?;

    FieldMetadataType::parse(&type_raw).ok_or_else(|| {
        GraphQlError::validation(format!(
            "Field \"type\" must be a valid FieldMetadataType, got {type_raw}"
        ))
    })?;

    Ok(())
}

pub fn normalize_update_object_payload(payload: &Value) -> Value {
    let Some(object) = payload.as_object() else {
        return json!({});
    };

    let mut result = Map::new();

    for key in comparable_properties(MetadataName::ObjectMetadata) {
        if let Some(value) = object.get(*key) {
            result.insert((*key).to_string(), value.clone());
        }
    }

    for key in [
        "labelIdentifierFieldMetadataId",
        "imageIdentifierFieldMetadataId",
    ] {
        if let Some(value) = object.get(key) {
            result.insert(key.to_string(), value.clone());
        }
    }

    Value::Object(result)
}

pub fn normalize_update_field_payload(payload: &Value) -> Value {
    let Some(object) = payload.as_object() else {
        return json!({});
    };

    let mut result = Map::new();

    for key in comparable_properties(MetadataName::FieldMetadata) {
        if let Some(value) = object.get(*key) {
            result.insert((*key).to_string(), value.clone());
        }
    }

    if let Some(value) = object.get("settings").cloned() {
        result.insert("universalSettings".to_string(), value);
    }

    Value::Object(result)
}

pub struct ObjectCreateTranspilation {
    pub entity: UniversalFlatObjectMetadata,
    pub name_field: Option<UniversalFlatFieldMetadata>,
}

pub fn transpile_create_object(
    input: &Value,
    applications: &ResolvedApplications,
) -> Result<ObjectCreateTranspilation, GraphQlError> {
    let object = input
        .get("object")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GraphQlError::validation("Field \"object\" of type \"CreateObjectInput!\" is required.")
        })?;

    let mut sanitized = object.clone();

    sanitize_string_fields(
        &mut sanitized,
        &[
            "description",
            "icon",
            "labelPlural",
            "labelSingular",
            "namePlural",
            "nameSingular",
            "shortcut",
        ],
    );

    let name_singular = require_string(&sanitized, "nameSingular")?;
    let name_plural = require_string(&sanitized, "namePlural")?;
    let label_singular = require_string(&sanitized, "labelSingular")?;
    let label_plural = require_string(&sanitized, "labelPlural")?;

    let skip_name_field = optional_bool(&sanitized, "skipNameField").unwrap_or(false);
    let universal_identifier = optional_string(&sanitized, "universalIdentifier")?
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let application_universal_identifier = if optional_bool(&sanitized, "isRemote").unwrap_or(false)
    {
        applications.standard_universal_identifier.clone()
    } else {
        applications.custom_universal_identifier.clone()
    };

    let identifier_field_name = if skip_name_field { "id" } else { "name" };
    let label_identifier_field_metadata_universal_identifier = Some(field_universal_identifier(
        &application_universal_identifier,
        &universal_identifier,
        identifier_field_name,
    ));

    let now = Utc::now();

    let name_field = (!skip_name_field).then(|| UniversalFlatFieldMetadata {
        universal_identifier: label_identifier_field_metadata_universal_identifier
            .clone()
            .unwrap_or_default(),
        application_universal_identifier: application_universal_identifier.clone(),
        object_metadata_universal_identifier: universal_identifier.clone(),
        field_type: FieldMetadataType::Text,
        name: "name".to_string(),
        label: "Name".to_string(),
        description: Some("Name".to_string()),
        icon: Some("IconAbc".to_string()),
        is_active: true,
        is_nullable: true,
        is_searchable: true,
        is_audit_logged: true,
        created_at: now,
        updated_at: now,
        ..UniversalFlatFieldMetadata::default()
    });

    let entity = UniversalFlatObjectMetadata {
        universal_identifier,
        application_universal_identifier,
        name_singular,
        name_plural,
        label_singular: capitalize(&label_singular),
        label_plural: capitalize(&label_plural),
        description: optional_string(&sanitized, "description")?,
        icon: optional_string(&sanitized, "icon")?,
        color: optional_string(&sanitized, "color")?,
        shortcut: optional_string(&sanitized, "shortcut")?,
        is_remote: optional_bool(&sanitized, "isRemote").unwrap_or(false),
        is_active: true,
        is_system: false,
        is_searchable: true,
        is_audit_logged: true,
        is_ui_editable: true,
        is_ui_creatable: true,
        is_label_synced_with_name: optional_bool(&sanitized, "isLabelSyncedWithName")
            .unwrap_or(false),
        open_record_in: Some(ObjectOpenRecordIn::UserChoice),
        target_table_name: Some(DEPRECATED_TARGET_TABLE_NAME.to_string()),
        created_at: now,
        updated_at: now,
        label_identifier_field_metadata_universal_identifier,
        ..UniversalFlatObjectMetadata::default()
    };

    Ok(ObjectCreateTranspilation { entity, name_field })
}

pub fn sanitize_select_options_with_ids(options: &Value) -> Value {
    let Some(entries) = options.as_array() else {
        return Value::Array(Vec::new());
    };

    let prepared = entries
        .iter()
        .map(|entry| {
            let mut object = entry.as_object().cloned().unwrap_or_default();

            if !object.contains_key("id") {
                object.insert("id".to_string(), json!(Uuid::new_v4().to_string()));
            }

            Value::Object(object)
        })
        .collect::<Vec<Value>>();

    sanitize_select_options(&Value::Array(prepared))
}

pub struct CreateFieldTranspilation {
    pub field: UniversalFlatFieldMetadata,
    pub inverse: Option<UniversalFlatFieldMetadata>,
    pub indexes: Vec<UniversalFlatIndexMetadata>,
    pub options: Vec<UniversalFlatFieldMetadataOption>,
}

pub fn transpile_create_field(
    input: &Value,
    maps: &FlatWorkspaceMaps,
    applications: &ResolvedApplications,
) -> Result<CreateFieldTranspilation, GraphQlError> {
    let raw = input
        .get("field")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GraphQlError::validation("Field \"field\" of type \"CreateFieldInput!\" is required.")
        })?;

    let mut sanitized = raw.clone();

    sanitize_string_fields(
        &mut sanitized,
        &[
            "description",
            "icon",
            "label",
            "name",
            "objectMetadataId",
            "type",
        ],
    );

    if optional_bool(&sanitized, "isRemoteCreation").unwrap_or(false) {
        return Err(super::errors::field_transpilation_failure(
            "INVALID_FIELD_INPUT",
            "Remote fields aren't supported",
            "create",
            "Multiple validation errors occurred while creating field",
        ));
    }

    let object_metadata_id = require_string(&sanitized, "objectMetadataId")?;
    let object_metadata_id = require_uuid(&object_metadata_id, "objectMetadataId")?;

    let Some(parent_object) = maps.object_metadata_maps.find_by_id(&object_metadata_id) else {
        return Err(super::errors::field_transpilation_failure(
            "OBJECT_METADATA_NOT_FOUND",
            "Provided object metadata id does not exist",
            "create",
            "Multiple validation errors occurred while creating field",
        ));
    };

    let name = require_string(&sanitized, "name")?;
    let label = require_string(&sanitized, "label")?;

    let type_raw = require_string(&sanitized, "type")?;
    let field_type = FieldMetadataType::parse(&type_raw)
        .ok_or_else(|| GraphQlError::validation(format!("Invalid field type: {type_raw}")))?;

    if field_type == FieldMetadataType::TsVector {
        return Err(super::errors::field_transpilation_failure(
            "INVALID_FIELD_INPUT",
            "TS Vector is not supported for field creation",
            "create",
            "Multiple validation errors occurred while creating field",
        ));
    }

    let now = Utc::now();
    let universal_identifier = optional_string(&sanitized, "universalIdentifier")?
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let options = match sanitized.get("options") {
        Some(value) if !value.is_null() => Some(sanitize_select_options_with_ids(value)),
        _ if field_type == FieldMetadataType::Rating => Some(rating_options()),
        _ => None,
    };

    let default_value = match sanitized.get("defaultValue") {
        Some(value) if !value.is_null() => Some(nullify_empty_composite_default_value(
            field_type.as_str(),
            value,
        )),
        _ => match field_type {
            FieldMetadataType::Actor => Some(json!({
                "source": "'MANUAL'",
                "name": "'System'",
                "workspaceMemberId": Value::Null
            })),
            _ => None,
        },
    };

    let settings = match sanitized.get("settings") {
        Some(value) if !value.is_null() => Some(value.clone()),
        _ => None,
    };

    let mut field = UniversalFlatFieldMetadata {
        universal_identifier,
        application_universal_identifier: applications.custom_universal_identifier.clone(),
        object_metadata_universal_identifier: parent_object.universal_identifier.clone(),
        field_type,
        name: name.clone(),
        label,
        description: optional_string(&sanitized, "description")?,
        icon: optional_string(&sanitized, "icon")?,
        is_active: true,
        is_system: optional_bool(&sanitized, "isSystem").unwrap_or(false),
        is_nullable: optional_bool(&sanitized, "isNullable").unwrap_or(true),
        is_unique: optional_bool(&sanitized, "isUnique").unwrap_or(false),
        is_searchable: optional_bool(&sanitized, "isSearchable").unwrap_or(false),
        is_audit_logged: optional_bool(&sanitized, "isAuditLogged")
            .unwrap_or(field_type != FieldMetadataType::Position),
        is_ui_editable: optional_bool(&sanitized, "isUIEditable")
            .or_else(|| {
                sanitized
                    .get("isUIReadOnly")
                    .and_then(Value::as_bool)
                    .map(|value| !value)
            })
            .unwrap_or(true),
        is_label_synced_with_name: optional_bool(&sanitized, "isLabelSyncedWithName")
            .unwrap_or(false),
        default_value,
        universal_settings: settings,
        options,
        created_at: now,
        updated_at: now,
        ..UniversalFlatFieldMetadata::default()
    };

    let mut inverse: Option<UniversalFlatFieldMetadata> = None;
    let mut indexes: Vec<UniversalFlatIndexMetadata> = Vec::new();

    if field_type == FieldMetadataType::Relation {
        let payload = sanitized
            .get("relationCreationPayload")
            .cloned()
            .filter(Value::is_object)
            .unwrap_or(Value::Null);

        if payload.is_null() {
            return Err(super::errors::field_transpilation_failure(
                "INVALID_FIELD_INPUT",
                "Relation creation payload is required",
                "create",
                "Multiple validation errors occurred while creating field",
            ));
        }

        let payload = payload.as_object().expect("checked object");
        let mut payload = payload.clone();

        sanitize_string_fields(
            &mut payload,
            &[
                "targetFieldIcon",
                "targetFieldLabel",
                "targetObjectMetadataId",
                "type",
            ],
        );

        let payload = &payload;

        let target_object_id = match require_string(payload, "targetObjectMetadataId")
            .and_then(|raw| require_uuid(&raw, "targetObjectMetadataId"))
        {
            Ok(raw) => raw,
            Err(_) => {
                return Err(super::errors::field_transpilation_failure(
                    "FIELD_METADATA_RELATION_MALFORMED",
                    "Relation creation payload is invalid: targetObjectMetadataId must be a UUID",
                    "create",
                    "Multiple validation errors occurred while creating field",
                ));
            }
        };

        let relation_type = match payload.get("type").and_then(Value::as_str) {
            Some(raw) => match crate::metadata::flat::RelationType::parse(raw) {
                Some(parsed) => parsed,
                None => {
                    return Err(super::errors::field_transpilation_failure(
                        "FIELD_METADATA_RELATION_MALFORMED",
                        "Relation creation payload is invalid: type must be a RelationType",
                        "create",
                        "Multiple validation errors occurred while creating field",
                    ));
                }
            },
            None => {
                return Err(super::errors::field_transpilation_failure(
                    "FIELD_METADATA_RELATION_MALFORMED",
                    "Relation creation payload is invalid: type must be a RelationType",
                    "create",
                    "Multiple validation errors occurred while creating field",
                ));
            }
        };

        let target_field_label = match payload.get("targetFieldLabel").and_then(Value::as_str) {
            Some(raw) if !raw.trim().is_empty() => raw.trim().to_string(),
            _ => {
                return Err(super::errors::field_transpilation_failure(
                    "FIELD_METADATA_RELATION_MALFORMED",
                    "Relation creation payload is invalid: targetFieldLabel must be a string",
                    "create",
                    "Multiple validation errors occurred while creating field",
                ));
            }
        };

        let Some(target_object) = maps.object_metadata_maps.find_by_id(&target_object_id) else {
            return Err(super::errors::field_transpilation_failure(
                "FIELD_METADATA_RELATION_MALFORMED",
                "Object metadata relation target not found for relation creation payload",
                "create",
                "Multiple validation errors occurred while creating field",
            ));
        };

        let target_field_name = compute_metadata_name_from_label(&target_field_label, true)
            .map_err(|_| {
                super::errors::field_transpilation_failure(
                    "INVALID_FIELD_INPUT",
                    "Relation target field label is invalid",
                    "create",
                    "Multiple validation errors occurred while creating field",
                )
            })?;

        let join_column = compute_relation_join_column_name(&name);

        field.universal_settings = Some(match relation_type {
            crate::metadata::flat::RelationType::ManyToOne => json!({
                "relationType": "MANY_TO_ONE",
                "onDelete": "SET_NULL",
                "joinColumnName": join_column
            }),
            _ => json!({ "relationType": relation_type_as_str(relation_type) }),
        });

        let target_field_universal_identifier = Uuid::new_v4().to_string();

        field.relation_target_object_metadata_universal_identifier =
            Some(target_object.universal_identifier.clone());
        field.relation_target_field_metadata_universal_identifier =
            Some(target_field_universal_identifier.clone());

        let target_join_column = compute_relation_join_column_name(&target_field_name);

        let inverse_settings = match relation_type {
            crate::metadata::flat::RelationType::ManyToOne => {
                json!({ "relationType": "ONE_TO_MANY" })
            }
            _ => json!({
                "relationType": "MANY_TO_ONE",
                "onDelete": "SET_NULL",
                "joinColumnName": target_join_column
            }),
        };

        let inverse_field = UniversalFlatFieldMetadata {
            universal_identifier: target_field_universal_identifier.clone(),
            application_universal_identifier: field.application_universal_identifier.clone(),
            object_metadata_universal_identifier: target_object.universal_identifier.clone(),
            field_type: FieldMetadataType::Relation,
            name: target_field_name,
            label: target_field_label,
            description: field.description.clone(),
            icon: Some("IconRelationOneToMany".to_string()),
            is_active: true,
            is_nullable: true,
            is_audit_logged: true,
            is_ui_editable: true,
            default_value: None,
            universal_settings: Some(inverse_settings),
            options: None,
            relation_target_object_metadata_universal_identifier: Some(
                parent_object.universal_identifier.clone(),
            ),
            relation_target_field_metadata_universal_identifier: Some(
                field.universal_identifier.clone(),
            ),
            created_at: now,
            updated_at: now,
            ..UniversalFlatFieldMetadata::default()
        };

        let (indexed_field, indexed_object) =
            if relation_type == crate::metadata::flat::RelationType::ManyToOne {
                (&field, parent_object)
            } else {
                (&inverse_field, target_object)
            };

        indexes.push(build_relation_backing_index(
            &flat_object_universal(indexed_object),
            indexed_field,
        ));

        inverse = Some(inverse_field);
    }

    let options = build_option_rows(&field);

    Ok(CreateFieldTranspilation {
        field,
        inverse,
        indexes,
        options,
    })
}

fn relation_type_as_str(relation_type: crate::metadata::flat::RelationType) -> &'static str {
    match relation_type {
        crate::metadata::flat::RelationType::OneToMany => "ONE_TO_MANY",
        crate::metadata::flat::RelationType::ManyToOne => "MANY_TO_ONE",
        crate::metadata::flat::RelationType::ManyToMany => "MANY_TO_MANY",
    }
}

pub fn build_option_rows(
    field: &UniversalFlatFieldMetadata,
) -> Vec<UniversalFlatFieldMetadataOption> {
    if !field.is_enum_type() {
        return Vec::new();
    }

    field
        .options
        .as_ref()
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    let object = option.as_object()?;
                    let id = object.get("id").and_then(Value::as_str)?;
                    let value = object.get("value").and_then(Value::as_str)?;

                    Some(UniversalFlatFieldMetadataOption {
                        universal_identifier: id.to_string(),
                        application_universal_identifier: field
                            .application_universal_identifier
                            .clone(),
                        field_metadata_universal_identifier: field.universal_identifier.clone(),
                        value: value.to_string(),
                        label: object
                            .get("label")
                            .and_then(Value::as_str)
                            .unwrap_or(value)
                            .to_string(),
                        color: object
                            .get("color")
                            .and_then(Value::as_str)
                            .filter(|raw| !raw.is_empty())
                            .or(Some("gray"))
                            .map(str::to_string),
                        description: object
                            .get("description")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        position: object
                            .get("position")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0),
                        is_active: object
                            .get("isActive")
                            .and_then(Value::as_bool)
                            .unwrap_or(true),
                        overrides: None,
                        created_at: field.created_at,
                        updated_at: field.updated_at,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn build_relation_backing_index(
    object: &UniversalFlatObjectMetadata,
    field: &UniversalFlatFieldMetadata,
) -> UniversalFlatIndexMetadata {
    let now = Utc::now();
    let index_universal_identifier = Uuid::new_v4().to_string();

    let column = compute_relation_join_column_name(&field.name);
    let table_name = compute_object_target_table(
        &object.name_singular,
        &object.application_universal_identifier,
    );

    UniversalFlatIndexMetadata {
        universal_identifier: index_universal_identifier.clone(),
        application_universal_identifier: field.application_universal_identifier.clone(),
        name: generate_deterministic_index_name(&table_name, &[column.as_str()], false, None),
        object_metadata_universal_identifier: object.universal_identifier.clone(),
        is_unique: field.is_unique,
        index_where_clause: None,
        index_type: IndexType::Btree,
        is_system_side_effect: true,
        created_at: now,
        updated_at: now,
        universal_flat_index_field_metadatas: vec![UniversalFlatIndexFieldMetadata {
            order: 0,
            sub_field_name: None,
            created_at: now,
            updated_at: now,
            index_metadata_universal_identifier: index_universal_identifier.clone(),
            field_metadata_universal_identifier: field.universal_identifier.clone(),
        }],
    }
}

pub fn build_unique_backing_index(
    object: &UniversalFlatObjectMetadata,
    field: &UniversalFlatFieldMetadata,
) -> UniversalFlatIndexMetadata {
    let now = Utc::now();
    let index_universal_identifier = Uuid::new_v4().to_string();

    let columns = index_columns_for_field(field);
    let column_refs: Vec<&str> = columns.iter().map(String::as_str).collect();

    let table_name = match &object.target_table_name {
        Some(name) if name != DEPRECATED_TARGET_TABLE_NAME => name.clone(),
        _ => compute_object_target_table(
            &object.name_singular,
            &object.application_universal_identifier,
        ),
    };

    UniversalFlatIndexMetadata {
        universal_identifier: index_universal_identifier.clone(),
        application_universal_identifier: field.application_universal_identifier.clone(),
        name: generate_deterministic_index_name(
            &table_name,
            &column_refs,
            true,
            Some("\"deletedAt\" IS NULL"),
        ),
        object_metadata_universal_identifier: object.universal_identifier.clone(),
        is_unique: true,
        index_where_clause: Some("\"deletedAt\" IS NULL".to_string()),
        index_type: IndexType::Btree,
        is_system_side_effect: true,
        created_at: now,
        updated_at: now,
        universal_flat_index_field_metadatas: vec![UniversalFlatIndexFieldMetadata {
            order: 0,
            sub_field_name: None,
            created_at: now,
            updated_at: now,
            index_metadata_universal_identifier: index_universal_identifier.clone(),
            field_metadata_universal_identifier: field.universal_identifier.clone(),
        }],
    }
}

pub fn index_columns_for_field(field: &UniversalFlatFieldMetadata) -> Vec<String> {
    match field.field_type {
        FieldMetadataType::Relation | FieldMetadataType::MorphRelation => {
            vec![compute_relation_join_column_name(&field.name)]
        }
        FieldMetadataType::MultiSelect => {
            vec![compute_composite_column_name(&field.name, "primary")]
        }
        _ => vec![field.name.clone()],
    }
}

pub fn index_name_for_columns(
    object: &UniversalFlatObjectMetadata,
    application_universal_identifier: &str,
    columns: &[String],
    is_unique: bool,
    where_clause: Option<&str>,
) -> String {
    let table_name = match &object.target_table_name {
        Some(name) if name != DEPRECATED_TARGET_TABLE_NAME => name.clone(),
        _ => compute_object_target_table(&object.name_singular, application_universal_identifier),
    };

    let column_refs: Vec<&str> = columns.iter().map(String::as_str).collect();

    generate_deterministic_index_name(&table_name, &column_refs, is_unique, where_clause)
}

pub fn merged_object_entity(
    existing: &UniversalFlatObjectMetadata,
    updates: &Value,
) -> UniversalFlatObjectMetadata {
    merge_into_existing(
        existing,
        updates,
        |entity| serde_json::to_value(entity).ok(),
        |payload| serde_json::from_value(payload).ok(),
    )
}

pub fn merged_field_entity(
    existing: &UniversalFlatFieldMetadata,
    updates: &Value,
) -> UniversalFlatFieldMetadata {
    merge_into_existing(
        existing,
        updates,
        |entity| serde_json::to_value(entity).ok(),
        |payload| serde_json::from_value(payload).ok(),
    )
}

fn merge_into_existing<E, ToValue, FromValue>(
    existing: &E,
    updates: &Value,
    to_value: ToValue,
    from_value: FromValue,
) -> E
where
    E: Clone,
    ToValue: Fn(&E) -> Option<Value>,
    FromValue: Fn(Value) -> Option<E>,
{
    let fallback = existing.clone();

    let Some(mut payload) = to_value(existing).and_then(|value| value.as_object().cloned()) else {
        return fallback;
    };

    if let Some(update) = updates.as_object() {
        for (key, value) in update {
            payload.insert(key.clone(), value.clone());
        }
    }

    from_value(Value::Object(payload)).unwrap_or(fallback)
}

pub fn translate_object_identifier_fields(
    mut updates: Value,
    existing_object: &FlatObjectMetadata,
    maps: &FlatWorkspaceMaps,
) -> Result<Value, GraphQlError> {
    let Some(object) = updates.as_object_mut() else {
        return Ok(updates);
    };

    if let Some(raw) = object.remove("labelIdentifierFieldMetadataId") {
        if raw.is_null() {
            return Err(
                GraphQlError::user_input("Field label identifier is required")
                    .with_extensions(json!({ "subCode": "INVALID_OBJECT_INPUT" })),
            );
        }

        let identifier = raw.as_str().ok_or_else(|| {
            GraphQlError::validation("labelIdentifierFieldMetadataId must be a string")
        })?;

        let field_id = require_uuid(identifier, "labelIdentifierFieldMetadataId")?;

        let Some(field) = maps.field_metadata_maps.find_by_id(&field_id) else {
            return Err(
                GraphQlError::user_input("Field declared as label identifier not found")
                    .with_extensions(json!({ "subCode": "INVALID_OBJECT_INPUT" })),
            );
        };

        if field.object_metadata_id != existing_object.id {
            return Err(GraphQlError::user_input(
                "Field declared as label identifier does not belong to this object",
            )
            .with_extensions(json!({ "subCode": "INVALID_OBJECT_INPUT" })));
        }

        if !matches!(
            field.field_type,
            FieldMetadataType::Uuid | FieldMetadataType::Text | FieldMetadataType::FullName
        ) {
            return Err(GraphQlError::user_input(
                "Field cannot be used as label identifier due to its type: should be of type UUID, text or full name",
            )
            .with_extensions(json!({ "subCode": "INVALID_OBJECT_INPUT" })));
        }

        object.insert(
            "labelIdentifierFieldMetadataUniversalIdentifier".to_string(),
            Value::String(field.universal_identifier.clone()),
        );
    }

    if object.contains_key("imageIdentifierFieldMetadataId") {
        let raw = object.remove("imageIdentifierFieldMetadataId");

        if let Some(Value::String(identifier)) = raw {
            let field_id = require_uuid(&identifier, "imageIdentifierFieldMetadataId")?;

            let Some(field) = maps.field_metadata_maps.find_by_id(&field_id) else {
                return Err(GraphQlError::user_input(
                    "Field declared as image identifier not found",
                )
                .with_extensions(json!({ "subCode": "INVALID_OBJECT_INPUT" })));
            };

            if field.object_metadata_id != existing_object.id {
                return Err(GraphQlError::user_input(
                    "Field declared as image identifier does not belong to this object",
                )
                .with_extensions(json!({ "subCode": "INVALID_OBJECT_INPUT" })));
            }

            if !matches!(
                field.field_type,
                FieldMetadataType::Files | FieldMetadataType::Links
            ) {
                return Err(GraphQlError::user_input(
                    "Field cannot be used as image identifier due to its type: should be of type Files or Links",
                )
                .with_extensions(json!({ "subCode": "INVALID_OBJECT_INPUT" })));
            }

            object.insert(
                "imageIdentifierFieldMetadataUniversalIdentifier".to_string(),
                Value::String(field.universal_identifier.clone()),
            );
        }
    }

    Ok(Value::Object(object.clone()))
}

pub fn resolve_target_object(
    value: &str,
    maps: &FlatEntityMaps<FlatObjectMetadata>,
) -> Result<FlatObjectMetadata, GraphQlError> {
    if let Ok(id) = Uuid::parse_str(value) {
        if let Some(found) = maps.find_by_id(&id.to_string()) {
            return Ok(found.clone());
        }
    }

    maps.find_by_universal_identifier(value)
        .cloned()
        .ok_or_else(|| GraphQlError::not_found(format!("Object with id {value} not found")))
}

pub fn resolve_target_field(
    value: &str,
    maps: &FlatEntityMaps<FlatFieldMetadata>,
) -> Result<FlatFieldMetadata, GraphQlError> {
    if let Ok(id) = Uuid::parse_str(value) {
        if let Some(found) = maps.find_by_id(&id.to_string()) {
            return Ok(found.clone());
        }
    }

    maps.find_by_universal_identifier(value)
        .cloned()
        .ok_or_else(|| GraphQlError::not_found(format!("Field with id {value} not found")))
}

pub fn flat_object_universal(object: &FlatObjectMetadata) -> UniversalFlatObjectMetadata {
    object.clone().universal()
}

pub fn flat_field_universal(field: &FlatFieldMetadata) -> UniversalFlatFieldMetadata {
    field.clone().universal()
}

pub fn flat_index_universal(index: &FlatIndexMetadata) -> UniversalFlatIndexMetadata {
    UniversalFlatIndexMetadata {
        universal_identifier: index.universal_identifier.clone(),
        application_universal_identifier: String::new(),
        name: index.name.clone(),
        object_metadata_universal_identifier: index.object_metadata_universal_identifier.clone(),
        is_unique: index.is_unique,
        index_where_clause: index.index_where_clause.clone(),
        index_type: index.index_type,
        is_system_side_effect: index.is_system_side_effect,
        created_at: index.created_at,
        updated_at: index.updated_at,
        universal_flat_index_field_metadatas: index.universal_flat_index_field_metadatas.clone(),
    }
}

pub fn flat_option_universal(option: &FlatFieldMetadataOption) -> UniversalFlatFieldMetadataOption {
    option.clone().universal()
}

pub enum OptionOperation {
    Create(UniversalFlatFieldMetadataOption),
    Update(UniversalFlatFieldMetadataOption),
    Delete(UniversalFlatFieldMetadataOption),
}

pub fn option_operations_for_update(
    merged_field: &UniversalFlatFieldMetadata,
    maps: &FlatWorkspaceMaps,
) -> Vec<OptionOperation> {
    if !merged_field.is_enum_type() {
        return Vec::new();
    }

    let existing_rows: Vec<&FlatFieldMetadataOption> = maps
        .field_metadata_option_maps
        .by_universal_identifier
        .values()
        .filter(|option| option.field_metadata_id == maps_field_id(merged_field, maps))
        .collect();

    let target_rows = build_option_rows(merged_field);

    let mut operations = Vec::new();

    for existing in &existing_rows {
        if !target_rows
            .iter()
            .any(|target| target.universal_identifier == existing.universal_identifier)
        {
            operations.push(OptionOperation::Delete(flat_option_universal(existing)));
        }
    }

    for target in &target_rows {
        match existing_rows
            .iter()
            .find(|existing| existing.universal_identifier == target.universal_identifier)
        {
            None => operations.push(OptionOperation::Create(target.clone())),
            Some(existing) => {
                let changed = existing.value != target.value
                    || existing.label != target.label
                    || existing.color != target.color
                    || existing.description != target.description
                    || (existing.position - target.position).abs() > f64::EPSILON
                    || existing.is_active != target.is_active;

                if changed {
                    operations.push(OptionOperation::Update(target.clone()));
                }
            }
        }
    }

    operations
}

fn maps_field_id(field: &UniversalFlatFieldMetadata, maps: &FlatWorkspaceMaps) -> Uuid {
    maps.field_metadata_maps
        .find_by_universal_identifier(&field.universal_identifier)
        .map(|entry| entry.id)
        .unwrap_or_else(Uuid::nil)
}

pub fn format_timestamp(timestamp: chrono::DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn object_dto(object: &FlatObjectMetadata) -> Value {
    json!({
        "id": object.id.to_string(),
        "universalIdentifier": object.universal_identifier,
        "nameSingular": object.name_singular,
        "namePlural": object.name_plural,
        "labelSingular": object.label_singular,
        "labelPlural": object.label_plural,
        "description": object.description.clone().unwrap_or_default(),
        "icon": object.icon.clone().unwrap_or_default(),
        "color": object.color.clone().unwrap_or_default(),
        "shortcut": object.shortcut.clone().unwrap_or_default(),
        "isActive": object.is_active,
        "isRemote": object.is_remote,
        "isSystem": object.is_system,
        "isSearchable": object.is_searchable,
        "isAuditLogged": object.is_audit_logged,
        "isLabelSyncedWithName": object.is_label_synced_with_name,
        "isUIEditable": object.is_ui_editable,
        "isUICreatable": object.is_ui_creatable,
        "isUIReadOnly": !object.is_ui_editable,
        "openRecordIn": object.open_record_in.map(|value| value.as_str()),
        "disabledObjectPermissionActions": object.disabled_object_permission_actions.clone(),
        "workspaceId": object.workspace_id.to_string(),
        "applicationId": object.application_id.to_string(),
        "labelIdentifierFieldMetadataId": object
            .label_identifier_field_metadata_universal_identifier
            .clone()
            .unwrap_or_default(),
        "imageIdentifierFieldMetadataId": object
            .image_identifier_field_metadata_universal_identifier
            .clone(),
        "createdAt": format_timestamp(object.created_at),
        "updatedAt": format_timestamp(object.updated_at)
    })
}

pub fn field_dto(field: &FlatFieldMetadata, maps: &FlatWorkspaceMaps) -> Value {
    let mut options = field.options.clone();

    if field.field_type.is_enum_type() && !options.as_ref().is_some_and(Value::is_array) {
        let rows: Vec<Value> = maps
            .field_metadata_option_maps
            .by_universal_identifier
            .values()
            .filter(|option| option.field_metadata_id == field.id)
            .map(option_row_dto)
            .collect();

        if !rows.is_empty() {
            options = Some(Value::Array(rows));
        }
    }

    json!({
        "id": field.id.to_string(),
        "universalIdentifier": field.universal_identifier,
        "type": field.field_type.as_str(),
        "name": field.name,
        "label": field.label,
        "description": field.description.clone().unwrap_or_default(),
        "icon": field.icon.clone().unwrap_or_default(),
        "isActive": field.is_active,
        "isSystem": field.is_system,
        "isSystemSideEffect": field.is_system_side_effect,
        "isNullable": field.is_nullable,
        "isUnique": field.is_unique,
        "isSearchable": field.is_searchable,
        "isAuditLogged": field.is_audit_logged,
        "isLabelSyncedWithName": field.is_label_synced_with_name,
        "isUIEditable": field.is_ui_editable,
        "isUIReadOnly": !field.is_ui_editable,
        "defaultValue": field.default_value.clone(),
        "options": options,
        "settings": field.universal_settings.clone().or_else(|| field.settings.clone()),
        "objectMetadataId": field.object_metadata_id.to_string(),
        "relationTargetFieldMetadataId": field
            .relation_target_field_metadata_universal_identifier
            .clone(),
        "relationTargetObjectMetadataId": field
            .relation_target_object_metadata_universal_identifier
            .clone(),
        "morphId": field.morph_id.map(|value| value.to_string()),
        "workspaceId": field.workspace_id.to_string(),
        "applicationId": field.application_id.to_string(),
        "createdAt": format_timestamp(field.created_at),
        "updatedAt": format_timestamp(field.updated_at)
    })
}

pub fn option_row_dto(option: &FlatFieldMetadataOption) -> Value {
    json!({
        "id": option.id.to_string(),
        "universalIdentifier": option.universal_identifier,
        "label": option.label,
        "value": option.value,
        "position": option.position,
        "color": option.color.clone().unwrap_or_else(|| "gray".to_string()),
        "isActive": option.is_active,
        "description": option.description.clone()
    })
}

pub fn index_dto(index: &FlatIndexMetadata) -> Value {
    json!({
        "id": index.id.to_string(),
        "name": index.name,
        "isUnique": index.is_unique,
        "indexWhereClause": index.index_where_clause.clone(),
        "indexType": index.index_type.as_str(),
        "objectMetadataId": index.object_metadata_id.to_string(),
        "workspaceId": index.workspace_id.to_string(),
        "applicationId": index.application_id.to_string(),
        "isSystemSideEffect": index.is_system_side_effect,
        "createdAt": format_timestamp(index.created_at),
        "updatedAt": format_timestamp(index.updated_at),
        "indexFieldMetadatas": index
            .flat_index_field_metadatas
            .iter()
            .map(|field| {
                json!({
                    "id": field.id.to_string(),
                    "order": field.order,
                    "subFieldName": field.sub_field_name.clone(),
                    "fieldMetadataId": field.field_metadata_id.to_string()
                })
            })
            .collect::<Vec<Value>>()
    })
}

pub fn project(base: &Value, selection: &[FieldSelection]) -> Value {
    if selection.is_empty() {
        return base.clone();
    }

    if let Value::Array(items) = base {
        return Value::Array(items.iter().map(|item| project(item, selection)).collect());
    }

    let mut result = Map::new();

    for field in selection {
        let key = field.response_key().to_string();

        let value = match base.get(field.name.as_str()) {
            Some(child) => match child {
                Value::Array(items) if !field.selection.is_empty() => Value::Array(
                    items
                        .iter()
                        .map(|item| project(item, &field.selection))
                        .collect(),
                ),
                Value::Object(_) if !field.selection.is_empty() => project(child, &field.selection),
                other => other.clone(),
            },
            None => Value::Null,
        };

        result.insert(key, value);
    }

    Value::Object(result)
}

pub fn input_argument(selection: &FieldSelection, variables: &Value) -> Value {
    match selection.argument("input") {
        Some(value) if value.is_object() => value.clone(),
        _ => variables
            .get("input")
            .cloned()
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({})),
    }
}

pub fn inputs_argument(selection: &FieldSelection, variables: &Value) -> Vec<Value> {
    for candidate in [
        selection.argument("inputs").cloned(),
        selection.argument("input").cloned(),
        variables.get("inputs").cloned(),
        variables.get("input").cloned(),
    ]
    .into_iter()
    .flatten()
    {
        match candidate {
            Value::Array(items) => return items,
            Value::Object(_) => return vec![candidate],
            _ => continue,
        }
    }

    Vec::new()
}

pub fn builder_failure_message_for(
    report: &OrchestratorFailureReport,
    fallback: &str,
) -> GraphQlError {
    GraphQlError::builder_failure(fallback, report)
}

fn retyped_create_entity(
    action: &mut crate::metadata::actions::UniversalCreateAction,
) -> Option<crate::metadata::actions::MetadataName> {
    let metadata_name = action.metadata_name;

    let Some(UniversalFlatEntity::Other { entity, .. }) = &action.flat_entity else {
        return Some(metadata_name);
    };

    if !matches!(
        metadata_name,
        MetadataName::ObjectMetadata
            | MetadataName::FieldMetadata
            | MetadataName::Index
            | MetadataName::FieldMetadataOption
            | MetadataName::SearchFieldMetadata
    ) {
        return Some(metadata_name);
    }

    match UniversalFlatEntity::from_value(metadata_name, entity.clone()) {
        Ok(typed) => {
            action.flat_entity = Some(typed);
            Some(metadata_name)
        }
        Err(_) => None,
    }
}

pub fn normalize_create_action_entities(
    actions: &mut Vec<crate::metadata::actions::UniversalAction>,
) {
    use crate::metadata::actions::UniversalAction;

    for action in actions.iter_mut() {
        let UniversalAction::Create(create) = action else {
            continue;
        };

        if retyped_create_entity(create).is_none() {
            continue;
        }

        if create.id.is_none() {
            create.id = Some(Uuid::new_v4().to_string());
        }
    }

    fold_fields_into_object_creates(actions);
}

fn fold_fields_into_object_creates(actions: &mut Vec<crate::metadata::actions::UniversalAction>) {
    use crate::metadata::actions::UniversalAction;

    let mut object_positions: Vec<(usize, String)> = Vec::new();

    for (position, action) in actions.iter().enumerate() {
        let UniversalAction::Create(create) = action else {
            continue;
        };

        if create.metadata_name != MetadataName::ObjectMetadata {
            continue;
        }

        let Some(UniversalFlatEntity::ObjectMetadata(object)) = &create.flat_entity else {
            continue;
        };

        object_positions.push((position, object.universal_identifier.clone()));
    }

    if object_positions.is_empty() {
        return;
    }

    let mut folds: Vec<(usize, usize, UniversalFlatFieldMetadata, Option<String>)> = Vec::new();

    for (position, action) in actions.iter().enumerate() {
        let UniversalAction::Create(create) = action else {
            continue;
        };

        if create.metadata_name != MetadataName::FieldMetadata {
            continue;
        }

        let Some(UniversalFlatEntity::FieldMetadata(field)) = &create.flat_entity else {
            continue;
        };

        if field.field_type.is_relation_or_morph() {
            continue;
        }

        let Some((object_position, _)) = object_positions
            .iter()
            .find(|(_, identifier)| identifier == &field.object_metadata_universal_identifier)
        else {
            continue;
        };

        folds.push((
            *object_position,
            position,
            (**field).clone(),
            create.id.clone(),
        ));
    }

    let removals: std::collections::HashSet<usize> =
        folds.iter().map(|(_, position, _, _)| *position).collect();

    for (object_position, _, field, field_id) in folds {
        let UniversalAction::Create(object_create) = &mut actions[object_position] else {
            continue;
        };

        if let Some(id) = field_id {
            object_create
                .field_id_by_universal_identifier
                .get_or_insert_with(Default::default)
                .insert(field.universal_identifier.clone(), id);
        }

        object_create.universal_flat_field_metadatas.push(field);
    }

    let kept: Vec<crate::metadata::actions::UniversalAction> = actions
        .iter()
        .enumerate()
        .filter(|(position, _)| !removals.contains(position))
        .map(|(_, action)| action.clone())
        .collect();

    actions.clone_from(&kept);
}

#[cfg(test)]
mod db_tests {
    use nudge_core::NodeEnv;
    use uuid::Uuid;

    use super::*;
    use crate::metadata::api::graphql::FieldSelection;
    use crate::metadata::api::handlers::dispatch;
    use crate::metadata::api::handlers::ResponsePayload;
    use crate::state::AppState;

    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn test_config(pg_database_url: String) -> nudge_core::Config {
        nudge_core::Config {
            node_env: NodeEnv::Test,
            node_port: 3999,
            server_url: "http://localhost:3999".to_string(),
            pg_database_url,
            pg_pool_max_connections: 4,
            redis_url: "redis://127.0.0.1:6399".to_string(),
            redis_cache_prefix: String::new(),
            app_secret: "integration-test-secret".to_string(),
            encryption_key: None,
            fallback_encryption_key: None,
            access_token_expires_in_secs: 1800,
            refresh_token_expires_in_secs: 3600,
            login_token_expires_in_secs: 900,
            workspace_agnostic_token_expires_in_secs: 1800,
            refresh_token_reuse_grace_period_secs: 60,
            frontend_url: "http://localhost:3999".to_string(),
            auth_cookie_same_site: "lax".to_string(),
            auth_cookie_allowed_origins: String::new(),
            session_absolute_lifetime_secs: 3600,
            session_idle_timeout_secs: 3600,
            is_cloudflare_geolocation_enabled: false,
            workspace_schema_ddl_locked: false,
            search_ilike_fallback_timeout_ms: 2000,
        }
    }

    async fn test_state(url: &str) -> AppState {
        let state = AppState::connect(&test_config(url.to_string()))
            .await
            .expect("connecting the test state");

        crate::workspace_migrations::ddl::test_support::migrate_core(url)
            .await
            .expect("running core migrations");

        state
    }

    async fn create_workspace(state: &AppState) -> (Uuid, String) {
        let workspace_id = Uuid::new_v4();
        let application_id = Uuid::new_v4();
        let application_universal_identifier = Uuid::new_v4().to_string();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let role_id = Uuid::new_v4();
        let subdomain = format!("api-{}", &Uuid::new_v4().simple().to_string()[..10]);

        let mut transaction = state.core_pool.begin().await.expect("fixture tx");

        sqlx::query(
            "INSERT INTO core.\"workspace\" \
             (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
              \"workspaceCustomApplicationId\", \"isPasswordAuthEnabled\", \"databaseSchema\") \
             VALUES ($1, 'API Test', $2, 'PENDING_CREATION', $3, $4, true, $5)",
        )
        .bind(workspace_id)
        .bind(Uuid::new_v4())
        .bind(&subdomain)
        .bind(application_id)
        .bind(&schema_name)
        .execute(&mut *transaction)
        .await
        .expect("inserting the workspace");

        sqlx::query(
            "INSERT INTO core.\"application\" \
             (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", \
              \"workspaceId\") \
             VALUES ($1, $2, 'API Test Application', 'LOCAL', 'INSTALLED', '', $3)",
        )
        .bind(application_id)
        .bind(
            Uuid::parse_str(&application_universal_identifier)
                .expect("application universal identifier is a uuid"),
        )
        .bind(workspace_id)
        .execute(&mut *transaction)
        .await
        .expect("inserting the application");

        sqlx::query(
            "INSERT INTO core.\"role\" \
             (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\") \
             VALUES ($1, $2, 'Member', $3, $4)",
        )
        .bind(role_id)
        .bind(Uuid::new_v4())
        .bind(application_id)
        .bind(workspace_id)
        .execute(&mut *transaction)
        .await
        .expect("inserting the default role");

        sqlx::query(
            "UPDATE core.\"workspace\" SET \"activationStatus\" = 'CREATED', \
             \"defaultRoleId\" = $2 WHERE id = $1",
        )
        .bind(workspace_id)
        .bind(role_id)
        .execute(&mut *transaction)
        .await
        .expect("activating the workspace");

        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\""))
            .execute(&mut *transaction)
            .await
            .expect("creating the workspace schema");

        transaction.commit().await.expect("committing the fixture");

        (workspace_id, schema_name)
    }

    fn selection(name: &str, input: Value) -> FieldSelection {
        FieldSelection {
            alias: None,
            name: name.to_string(),
            line: 1,
            column: 1,
            arguments: vec![("input".to_string(), input)],
            selection: Vec::new(),
            spreads: Vec::new(),
        }
    }

    async fn run_operation(
        state: &AppState,
        workspace_id: Uuid,
        name: &str,
        input: Value,
    ) -> ResponsePayload {
        let selection = selection(name, input);
        let variables = json!({});

        let ctx = OperationContext {
            state,
            workspace_id,
            auth: None,
            selection: &selection,
            variables: &variables,
            is_query: false,
        };

        dispatch(&ctx).await
    }

    fn node(payload: &ResponsePayload, name: &str) -> Value {
        payload
            .data
            .as_ref()
            .and_then(|data| data.get(name))
            .cloned()
            .unwrap_or(Value::Null)
    }

    async fn metadata_version(state: &AppState, workspace_id: Uuid) -> i64 {
        let row: (i64,) = sqlx::query_as(
            "SELECT \"metadataVersion\"::bigint FROM core.\"workspace\" WHERE id = $1",
        )
        .bind(workspace_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("reading metadata version");

        row.0
    }

    async fn column_exists(state: &AppState, schema: &str, table: &str, column: &str) -> bool {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
             WHERE table_schema = $1 AND table_name = $2 AND column_name = $3)",
        )
        .bind(schema)
        .bind(table)
        .bind(column)
        .fetch_one(&state.core_pool)
        .await
        .expect("checking column existence");

        row.0
    }

    async fn object_row(state: &AppState, workspace_id: Uuid, name_singular: &str) -> Value {
        let row: (Uuid, String, Uuid, Option<Uuid>) = sqlx::query_as(
            "SELECT id, \"targetTableName\", \"universalIdentifier\", \
             \"labelIdentifierFieldMetadataId\" \
             FROM core.\"objectMetadata\" WHERE \"workspaceId\" = $1 AND \"nameSingular\" = $2",
        )
        .bind(workspace_id)
        .bind(name_singular)
        .fetch_one(&state.core_pool)
        .await
        .unwrap_or_else(|error| panic!("reading the {name_singular} object: {error}"));

        json!({
            "id": row.0.to_string(),
            "targetTableName": row.1,
            "universalIdentifier": row.2.to_string(),
            "labelIdentifierFieldMetadataId": row.3.map(|value| value.to_string())
        })
    }

    async fn count_bound(state: &AppState, sql: &str, first: Uuid, second: Uuid) -> i64 {
        let row: (i64,) = sqlx::query_as(sql)
            .bind(first)
            .bind(second)
            .fetch_one(&state.core_pool)
            .await
            .unwrap_or_else(|error| panic!("counting rows: {error}"));

        row.0
    }

    async fn index_names_for_field(state: &AppState, field_id: Uuid) -> Vec<String> {
        sqlx::query_as::<_, (String,)>(
            "SELECT im.name FROM core.\"indexFieldMetadata\" ifm \
             JOIN core.\"indexMetadata\" im ON im.id = ifm.\"indexMetadataId\" \
             WHERE ifm.\"fieldMetadataId\" = $1",
        )
        .bind(field_id)
        .fetch_all(&state.core_pool)
        .await
        .expect("reading index names")
        .into_iter()
        .map(|row| row.0)
        .collect()
    }

    async fn search_vector_generation_expression(state: &AppState, schema: &str) -> Option<String> {
        sqlx::query_as::<_, (String,)>(
            "SELECT generation_expression FROM information_schema.columns \
             WHERE table_schema = $1 AND table_name = '_gadget' AND column_name = 'searchVector'",
        )
        .bind(schema)
        .fetch_optional(&state.core_pool)
        .await
        .expect("reading the searchVector expression")
        .map(|row| row.0)
    }

    #[tokio::test]
    async fn create_one_object_persists_object_and_name_field_end_to_end() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };

        let _guard = TEST_LOCK.lock().await;

        let state = test_state(&url).await;
        let (workspace_id, schema_name) = create_workspace(&state).await;

        let before = metadata_version(&state, workspace_id).await;

        let payload = run_operation(
            &state,
            workspace_id,
            "createOneObject",
            json!({
                "object": {
                    "nameSingular": "rocket",
                    "namePlural": "rockets",
                    "labelSingular": "Rocket",
                    "labelPlural": "Rockets"
                }
            }),
        )
        .await;

        assert!(
            payload.errors.is_empty(),
            "createOneObject failed: {}",
            payload.to_json()
        );

        let created = node(&payload, "createOneObject");

        assert_eq!(created["nameSingular"], json!("rocket"));
        assert_eq!(created["labelPlural"], json!("Rockets"));

        let row = object_row(&state, workspace_id, "rocket").await;

        assert_eq!(row["targetTableName"], json!("DEPRECATED"));

        let label_identifier = row["labelIdentifierFieldMetadataId"]
            .as_str()
            .expect("the object resolves its name field id");

        let name_field: (String, Uuid) = sqlx::query_as(
            "SELECT name, \"universalIdentifier\" FROM core.\"fieldMetadata\" WHERE id = $1",
        )
        .bind(Uuid::parse_str(label_identifier).expect("label identifier is a uuid"))
        .fetch_one(&state.core_pool)
        .await
        .expect("reading the label identifier field");

        assert_eq!(name_field.0, "name");
        assert_eq!(
            name_field.1.to_string(),
            created["labelIdentifierFieldMetadataId"]
                .as_str()
                .expect("the dto exposes the label identifier")
        );

        let field_count = count_bound(
            &state,
            "SELECT count(*)::bigint FROM core.\"fieldMetadata\" f \
             JOIN core.\"objectMetadata\" o ON o.id = f.\"objectMetadataId\" \
             WHERE o.id = $1 AND o.\"workspaceId\" = $2",
            Uuid::parse_str(row["id"].as_str().unwrap()).expect("object id"),
            workspace_id,
        )
        .await;

        assert_eq!(field_count, 1, "exactly the name field is seeded");

        let rocket_object_id = Uuid::parse_str(row["id"].as_str().unwrap()).expect("object id");

        let view_count = count_bound(
            &state,
            "SELECT count(*)::bigint FROM core.view v \
             WHERE v.\"objectMetadataId\" = $1 AND v.\"workspaceId\" = $2",
            rocket_object_id,
            workspace_id,
        )
        .await;

        let navigation_count = count_bound(
            &state,
            "SELECT count(*)::bigint FROM core.\"navigationMenuItem\" n \
             WHERE n.\"targetObjectMetadataId\" = $1 AND n.\"workspaceId\" = $2",
            rocket_object_id,
            workspace_id,
        )
        .await;

        assert_eq!(view_count, 0, "no INDEX view is seeded in wave 3");
        assert_eq!(
            navigation_count, 0,
            "no navigation command item is seeded in wave 3"
        );

        assert!(
            column_exists(&state, &schema_name, "_rocket", "name").await,
            "the physical rocket table carries the name column"
        );
        assert!(
            column_exists(&state, &schema_name, "_rocket", "id").await,
            "the physical rocket table carries the base id column"
        );

        let after = metadata_version(&state, workspace_id).await;

        assert!(
            after > before,
            "metadataVersion bumped from {before} to {after}"
        );

        sqlx::query(&format!("DROP SCHEMA \"{schema_name}\" CASCADE"))
            .execute(&state.core_pool)
            .await
            .expect("dropping the workspace schema");
    }

    #[tokio::test]
    async fn delete_field_cascades_options_index_and_search_vector() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };

        let _guard = TEST_LOCK.lock().await;

        let state = test_state(&url).await;
        let (workspace_id, schema_name) = create_workspace(&state).await;

        let created = run_operation(
            &state,
            workspace_id,
            "createOneObject",
            json!({
                "object": {
                    "nameSingular": "gadget",
                    "namePlural": "gadgets",
                    "labelSingular": "Gadget",
                    "labelPlural": "Gadgets"
                }
            }),
        )
        .await;

        assert!(
            created.errors.is_empty(),
            "createOneObject failed: {}",
            created.to_json()
        );

        let gadget = object_row(&state, workspace_id, "gadget").await;
        let gadget_id = gadget["id"].as_str().unwrap().to_string();
        let gadget_uuid = Uuid::parse_str(&gadget_id).expect("gadget uuid");

        let status_field = run_operation(
            &state,
            workspace_id,
            "createOneField",
            json!({
                "field": {
                    "objectMetadataId": gadget_id,
                    "name": "status",
                    "label": "Status",
                    "type": "SELECT",
                    "isNullable": true,
                    "options": [
                        { "value": "DRAFT", "label": "Draft", "color": "gray", "position": 0 },
                        { "value": "LIVE", "label": "Live", "color": "green", "position": 1 }
                    ]
                }
            }),
        )
        .await;

        assert!(
            status_field.errors.is_empty(),
            "createOneField status failed: {}",
            status_field.to_json()
        );

        let title_field = run_operation(
            &state,
            workspace_id,
            "createOneField",
            json!({
                "field": {
                    "objectMetadataId": gadget_id,
                    "name": "title",
                    "label": "Title",
                    "type": "TEXT",
                    "isNullable": true,
                    "isUnique": true
                }
            }),
        )
        .await;

        assert!(
            title_field.errors.is_empty(),
            "createOneField title failed: {}",
            title_field.to_json()
        );

        let status_node = node(&status_field, "createOneField");
        let status_uuid =
            Uuid::parse_str(status_node["id"].as_str().expect("status id")).expect("status uuid");
        let title_node = node(&title_field, "createOneField");
        let title_id =
            Uuid::parse_str(title_node["id"].as_str().expect("title id")).expect("title id");
        let title_universal = title_node["universalIdentifier"]
            .as_str()
            .expect("title universal identifier")
            .to_string();

        assert!(
            column_exists(&state, &schema_name, "_gadget", "title").await,
            "the title column exists before deletion"
        );

        let option_count = count_bound(
            &state,
            "SELECT count(*)::bigint FROM core.\"fieldMetadataOption\" \
             WHERE \"fieldMetadataId\" = $1 AND \"workspaceId\" = $2",
            status_uuid,
            workspace_id,
        )
        .await;

        assert_eq!(option_count, 2, "the select field carries two options");

        assert!(
            !index_names_for_field(&state, title_id).await.is_empty(),
            "the unique backing index carries an indexFieldMetadata row"
        );

        seed_search_vector_fixture(&state, &schema_name, gadget_uuid, title_id).await;

        crate::metadata::cache::invalidate_workspace_maps(&state, workspace_id).await;

        let deleted = run_operation(
            &state,
            workspace_id,
            "deleteOneField",
            json!({ "id": title_universal }),
        )
        .await;

        assert!(
            deleted.errors.is_empty(),
            "deleteOneField failed: {}",
            deleted.to_json()
        );

        assert!(
            !column_exists(&state, &schema_name, "_gadget", "title").await,
            "the title column is dropped"
        );
        assert!(
            column_exists(&state, &schema_name, "_gadget", "status").await,
            "the status column survives"
        );
        assert!(
            column_exists(&state, &schema_name, "_gadget", "searchVector").await,
            "the searchVector column is rebuilt after the cascade drop"
        );

        let search_rows = count_bound(
            &state,
            "SELECT count(*)::bigint FROM core.\"searchFieldMetadata\" \
             WHERE \"workspaceId\" = $1 AND \"fieldMetadataId\" = $2",
            workspace_id,
            title_id,
        )
        .await;

        assert_eq!(search_rows, 0, "the searchFieldMetadata row is gone");

        assert!(
            index_names_for_field(&state, title_id).await.is_empty(),
            "the indexFieldMetadata rows for the deleted field are gone"
        );

        let expression = search_vector_generation_expression(&state, &schema_name)
            .await
            .expect("the rebuilt searchVector column");

        assert!(
            !expression.contains("\"title\""),
            "the rebuilt searchVector no longer references the deleted column: {expression}"
        );
        assert!(
            expression.contains("searchVector") || expression.contains("name"),
            "the rebuilt searchVector keeps the remaining searchable column: {expression}"
        );

        let remaining_options = count_bound(
            &state,
            "SELECT count(*)::bigint FROM core.\"fieldMetadataOption\" \
             WHERE \"fieldMetadataId\" = $1 AND \"workspaceId\" = $2",
            status_uuid,
            workspace_id,
        )
        .await;

        assert_eq!(
            remaining_options, 2,
            "the surviving field keeps its options"
        );

        sqlx::query(&format!("DROP SCHEMA \"{schema_name}\" CASCADE"))
            .execute(&state.core_pool)
            .await
            .expect("dropping the workspace schema");
    }

    async fn seed_search_vector_fixture(
        state: &AppState,
        schema: &str,
        gadget_id: Uuid,
        title_id: Uuid,
    ) {
        let context: (Uuid, Uuid, Uuid) = sqlx::query_as(
            "SELECT o.\"workspaceId\", o.\"applicationId\", f.\"universalIdentifier\" \
             FROM core.\"objectMetadata\" o \
             JOIN core.\"fieldMetadata\" f ON f.\"objectMetadataId\" = o.id AND f.name = 'name' \
             WHERE o.id = $1",
        )
        .bind(gadget_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("reading the gadget workspace and application");

        let ts_vector_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO core.\"fieldMetadata\" \
             (id, \"objectMetadataId\", type, name, label, \"isActive\", \"isSystem\", \
              \"isNullable\", \"workspaceId\", \"universalIdentifier\", \"applicationId\", \
              \"isSystemSideEffect\") \
             VALUES ($1, $2, 'TS_VECTOR', 'searchVector', 'Search Vector', true, false, \
                     true, $3, $4, $5, true)",
        )
        .bind(ts_vector_id)
        .bind(gadget_id)
        .bind(context.0)
        .bind(ts_vector_id)
        .bind(context.1)
        .execute(&state.core_pool)
        .await
        .expect("inserting the ts vector field row");

        let name_id: (Uuid,) = sqlx::query_as(
            "SELECT f.id FROM core.\"fieldMetadata\" f \
             JOIN core.\"objectMetadata\" o ON o.id = f.\"objectMetadataId\" \
             WHERE o.id = $1 AND f.name = 'name'",
        )
        .bind(gadget_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("reading the name field id");

        for (position, field_id) in [(0.0, name_id.0), (1.0, title_id)] {
            let search_universal = Uuid::new_v4();

            sqlx::query(
                "INSERT INTO core.\"searchFieldMetadata\" \
                 (id, \"objectMetadataId\", \"fieldMetadataId\", \"workspaceId\", \
                  \"universalIdentifier\", \"applicationId\", \"position\", \
                  \"tsVectorFieldMetadataId\") \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(search_universal)
            .bind(gadget_id)
            .bind(field_id)
            .bind(context.0)
            .bind(search_universal)
            .bind(context.1)
            .bind(position)
            .bind(ts_vector_id)
            .execute(&state.core_pool)
            .await
            .expect("seeding a search field row");
        }

        sqlx::query(&format!(
            "ALTER TABLE \"{schema}\".\"_gadget\" ADD COLUMN \"searchVector\" tsvector \
             GENERATED ALWAYS AS (to_tsvector('simple', \
             COALESCE(public.unaccent_immutable(\"name\"), '') || ' ' || \
             COALESCE(public.unaccent_immutable(\"title\"), ''))) STORED"
        ))
        .execute(&state.core_pool)
        .await
        .expect("adding the searchVector generated column");
    }
}
