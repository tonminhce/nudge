#![allow(dead_code)]

use std::collections::HashMap;
use std::collections::HashSet;

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::OrchestratorActionsReport;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalFlatEntity;
use crate::metadata::builders::BuildOptions;
use crate::metadata::builders::FailedFlatEntityValidation;
use crate::metadata::builders::FlatEntityValidationError;
use crate::metadata::builders::OrchestratorFailureReport;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FromToAllMaps;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalFlatFieldMetadata;
use crate::metadata::flat::UniversalFlatFieldMetadataOption;
use crate::metadata::flat::UniversalFlatObjectMetadata;
use crate::metadata::flat::UniversalFlatSearchFieldMetadata;
use crate::metadata::flat::UniversalWorkspaceMaps;

const LABEL_IDENTIFIER_FIELD_METADATA_TYPES: &[FieldMetadataType] = &[
    FieldMetadataType::Text,
    FieldMetadataType::FullName,
    FieldMetadataType::Uuid,
];

const SEARCHABLE_FIELD_TYPES: &[FieldMetadataType] = &[
    FieldMetadataType::Text,
    FieldMetadataType::FullName,
    FieldMetadataType::Emails,
    FieldMetadataType::Address,
    FieldMetadataType::Links,
    FieldMetadataType::Phones,
    FieldMetadataType::RichText,
    FieldMetadataType::Uuid,
];

pub fn cross_entity_transversal_validation(
    optimistic_maps: &UniversalWorkspaceMaps,
    actions_report: &OrchestratorActionsReport,
    from_to_maps: &FromToAllMaps,
    build_options: &BuildOptions,
) -> OrchestratorFailureReport {
    let mut cross_entity_failure_report = OrchestratorFailureReport::default();

    let object_failures = validate_object_metadata_cross_entity(optimistic_maps, actions_report);

    cross_entity_failure_report.push_all(MetadataName::ObjectMetadata, object_failures);

    let field_failures = validate_field_metadata_default_value_option_cross_entity(
        &from_to_maps.field_metadata_option.from,
        optimistic_maps,
        actions_report,
        build_options.application_universal_identifier
            == build_options.workspace_custom_application_universal_identifier,
    );

    cross_entity_failure_report.push_all(MetadataName::FieldMetadata, field_failures);

    validate_universal_identifier_cross_entity_uniqueness(
        optimistic_maps,
        actions_report,
        &mut cross_entity_failure_report,
    );

    cross_entity_failure_report
}

pub fn validate_universal_identifier_cross_entity_uniqueness(
    optimistic_maps: &UniversalWorkspaceMaps,
    actions_report: &OrchestratorActionsReport,
    failure_report: &mut OrchestratorFailureReport,
) {
    let mut all_universal_identifier_map: HashMap<String, (MetadataName, String)> = HashMap::new();

    for (metadata_name, entity) in owned_entities(optimistic_maps) {
        let application = entity
            .to_value()
            .ok()
            .and_then(|value| {
                value
                    .get("applicationUniversalIdentifier")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_default();

        all_universal_identifier_map.insert(
            entity.universal_identifier().to_string(),
            (metadata_name, application),
        );
    }

    for metadata_name in MetadataName::all() {
        let create_actions = &actions_report.actions_for(*metadata_name).create;

        if create_actions.is_empty() {
            continue;
        }

        for create_action in create_actions {
            let universal_identifier = match create_action.universal_identifier() {
                Some(universal_identifier) => universal_identifier.to_string(),
                None => continue,
            };

            let Some((existing_owner, existing_application)) =
                all_universal_identifier_map.get(&universal_identifier)
            else {
                continue;
            };

            if *existing_owner == *metadata_name {
                continue;
            }

            failure_report.push_all(
                *metadata_name,
                vec![FailedFlatEntityValidation {
                    validation_type: "create".to_string(),
                    metadata_name: Some(*metadata_name),
                    errors: vec![FlatEntityValidationError {
                        code: "ENTITY_ALREADY_EXISTS".to_string(),
                        message: format!(
                            "Cannot create {metadata_name}: universalIdentifier \"{universal_identifier}\" is already taken by {existing_owner} from application \"{existing_application}\""
                        ),
                        user_friendly_message: None,
                        value: None,
                    }],
                    flat_entity_minimal_information: serde_json::json!({
                        "universalIdentifier": universal_identifier
                    }),
                }],
            );
        }
    }
}

fn owned_entities(maps: &UniversalWorkspaceMaps) -> Vec<(MetadataName, UniversalFlatEntity)> {
    let mut entities: Vec<(MetadataName, UniversalFlatEntity)> = Vec::new();

    for object in maps.object_metadata_maps.by_universal_identifier.values() {
        entities.push((
            MetadataName::ObjectMetadata,
            UniversalFlatEntity::ObjectMetadata(Box::new(object.clone())),
        ));
    }

    for field in maps.field_metadata_maps.by_universal_identifier.values() {
        entities.push((
            MetadataName::FieldMetadata,
            UniversalFlatEntity::FieldMetadata(Box::new(field.clone())),
        ));
    }

    for index in maps.index_metadata_maps.by_universal_identifier.values() {
        entities.push((
            MetadataName::Index,
            UniversalFlatEntity::IndexMetadata(Box::new(index.clone())),
        ));
    }

    for option in maps
        .field_metadata_option_maps
        .by_universal_identifier
        .values()
    {
        entities.push((
            MetadataName::FieldMetadataOption,
            UniversalFlatEntity::FieldMetadataOption(Box::new(option.clone())),
        ));
    }

    for search_field in maps
        .search_field_metadata_maps
        .by_universal_identifier
        .values()
    {
        entities.push((
            MetadataName::SearchFieldMetadata,
            UniversalFlatEntity::SearchFieldMetadata(Box::new(search_field.clone())),
        ));
    }

    entities
}

pub fn validate_object_metadata_cross_entity(
    optimistic_maps: &UniversalWorkspaceMaps,
    actions_report: &OrchestratorActionsReport,
) -> Vec<FailedFlatEntityValidation> {
    let mut failures: Vec<FailedFlatEntityValidation> = Vec::new();

    let created_objects: Vec<&UniversalFlatObjectMetadata> = actions_report
        .actions_for(MetadataName::ObjectMetadata)
        .create
        .iter()
        .filter_map(|action| match action {
            UniversalAction::Create(create_action) => match create_action.flat_entity.as_ref()? {
                UniversalFlatEntity::ObjectMetadata(object) => Some(object.as_ref()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    for object in created_objects {
        let errors = validate_object_identifiers(object, &optimistic_maps.field_metadata_maps);

        if !errors.is_empty() {
            failures.push(FailedFlatEntityValidation {
                validation_type: "create".to_string(),
                metadata_name: Some(MetadataName::ObjectMetadata),
                errors,
                flat_entity_minimal_information: serde_json::to_value(object.minimal_information())
                    .unwrap_or(serde_json::Value::Null),
            });
        }
    }

    for action in &actions_report
        .actions_for(MetadataName::ObjectMetadata)
        .update
    {
        let Some(update_action) = action.clone().into_update() else {
            continue;
        };

        let label_identifier_touched = update_action
            .update
            .get("labelIdentifierFieldMetadataUniversalIdentifier")
            .is_some();

        if !label_identifier_touched {
            continue;
        }

        let Some(updated_object) = optimistic_maps
            .object_metadata_maps
            .find_by_universal_identifier(&update_action.universal_identifier)
        else {
            continue;
        };

        let errors =
            validate_object_identifiers(updated_object, &optimistic_maps.field_metadata_maps);

        if !errors.is_empty() {
            failures.push(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(MetadataName::ObjectMetadata),
                errors,
                flat_entity_minimal_information: serde_json::to_value(
                    updated_object.minimal_information(),
                )
                .unwrap_or(serde_json::Value::Null),
            });
        }
    }

    failures
}

pub fn validate_object_identifiers(
    object: &UniversalFlatObjectMetadata,
    field_maps: &UniversalFlatEntityMaps<UniversalFlatFieldMetadata>,
) -> Vec<FlatEntityValidationError> {
    let mut errors: Vec<FlatEntityValidationError> = Vec::new();

    if let Some(label_identifier) = &object.label_identifier_field_metadata_universal_identifier {
        match field_maps.find_by_universal_identifier(label_identifier) {
            None => {
                errors.push(FlatEntityValidationError {
                    code: "INVALID_OBJECT_INPUT".to_string(),
                    message: "labelIdentifierFieldMetadataUniversalIdentifier validation failed: related field metadata not found".to_string(),
                    user_friendly_message: Some("Field declared as label identifier not found".to_string()),
                    value: None,
                });
            }
            Some(field) => {
                if !LABEL_IDENTIFIER_FIELD_METADATA_TYPES.contains(&field.field_type) {
                    errors.push(FlatEntityValidationError {
                        code: "INVALID_OBJECT_INPUT".to_string(),
                        message: "labelIdentifierFieldMetadataUniversalIdentifier validation failed: field type not compatible".to_string(),
                        user_friendly_message: Some("Field cannot be used as label identifier due to its type: should be of type UUID, text or full name".to_string()),
                        value: None,
                    });
                } else if is_formula_field(field) {
                    errors.push(FlatEntityValidationError {
                        code: "INVALID_OBJECT_INPUT".to_string(),
                        message: "labelIdentifierFieldMetadataUniversalIdentifier validation failed: field is a formula".to_string(),
                        user_friendly_message: Some("The field used as record label cannot be a formula".to_string()),
                        value: None,
                    });
                } else if !SEARCHABLE_FIELD_TYPES.contains(&field.field_type) {
                    errors.push(FlatEntityValidationError {
                        code: "INVALID_OBJECT_INPUT".to_string(),
                        message: "labelIdentifierFieldMetadataUniversalIdentifier validation failed: field type not compatible".to_string(),
                        user_friendly_message: Some("Field cannot be used as label identifier due to its type".to_string()),
                        value: None,
                    });
                }
            }
        }
    }

    if let Some(image_identifier) = &object.image_identifier_field_metadata_universal_identifier {
        if field_maps
            .find_by_universal_identifier(image_identifier)
            .is_none()
        {
            errors.push(FlatEntityValidationError {
                code: "INVALID_OBJECT_INPUT".to_string(),
                message: "imageIdentifierFieldMetadataUniversalIdentifier validation failed: related field metadata not found".to_string(),
                user_friendly_message: Some("Field declared as image identifier not found".to_string()),
                value: None,
            });
        }
    }

    errors
}

fn is_formula_field(field: &UniversalFlatFieldMetadata) -> bool {
    field
        .universal_settings
        .as_ref()
        .map(|settings| settings.get("formula").is_some())
        .unwrap_or(false)
}

pub fn validate_field_metadata_default_value_option_cross_entity(
    from_flat_field_metadata_option_maps: &UniversalFlatEntityMaps<
        UniversalFlatFieldMetadataOption,
    >,
    optimistic_maps: &UniversalWorkspaceMaps,
    actions_report: &OrchestratorActionsReport,
    _is_workspace_caller: bool,
) -> Vec<FailedFlatEntityValidation> {
    let mut failures: Vec<FailedFlatEntityValidation> = Vec::new();

    let created_field_universal_identifiers: HashSet<String> = actions_report
        .actions_for(MetadataName::FieldMetadata)
        .create
        .iter()
        .filter_map(|action| action.universal_identifier().map(str::to_string))
        .collect();

    let mut touched_field_universal_identifiers: HashSet<String> =
        created_field_universal_identifiers.clone();

    for action in &actions_report
        .actions_for(MetadataName::FieldMetadata)
        .update
    {
        touched_field_universal_identifiers.insert(
            action
                .universal_identifier()
                .unwrap_or_default()
                .to_string(),
        );
    }

    for action in &actions_report
        .actions_for(MetadataName::FieldMetadataOption)
        .create
    {
        if let Some(entity) = action
            .clone()
            .into_create()
            .and_then(|create| create.flat_entity)
        {
            let owner = entity.to_value().ok().and_then(|value| {
                value
                    .get("fieldMetadataUniversalIdentifier")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            });

            if let Some(owner) = owner {
                touched_field_universal_identifiers.insert(owner);
            }
        }
    }

    for action in &actions_report
        .actions_for(MetadataName::FieldMetadataOption)
        .update
    {
        let owner = action
            .clone()
            .into_update()
            .and_then(|update| {
                optimistic_maps
                    .field_metadata_option_maps
                    .find_by_universal_identifier(&update.universal_identifier)
                    .cloned()
            })
            .map(|option| option.field_metadata_universal_identifier);

        if let Some(owner) = owner {
            touched_field_universal_identifiers.insert(owner);
        }
    }

    for action in &actions_report
        .actions_for(MetadataName::FieldMetadataOption)
        .delete
    {
        let owner = action
            .clone()
            .into_delete()
            .and_then(|delete| {
                from_flat_field_metadata_option_maps
                    .find_by_universal_identifier(&delete.universal_identifier)
                    .cloned()
            })
            .map(|option| option.field_metadata_universal_identifier);

        if let Some(owner) = owner {
            touched_field_universal_identifiers.insert(owner);
        }
    }

    if touched_field_universal_identifiers.is_empty() {
        return failures;
    }

    let mut option_values_by_field: HashMap<String, HashSet<String>> = HashMap::new();
    let mut inactive_option_values_by_field: HashMap<String, HashSet<String>> = HashMap::new();

    for option in optimistic_maps
        .field_metadata_option_maps
        .by_universal_identifier
        .values()
    {
        option_values_by_field
            .entry(option.field_metadata_universal_identifier.clone())
            .or_default()
            .insert(option.value.clone());

        if !option.is_active {
            inactive_option_values_by_field
                .entry(option.field_metadata_universal_identifier.clone())
                .or_default()
                .insert(option.value.clone());
        }
    }

    let field_universal_identifiers_with_default_value_update: HashSet<String> = actions_report
        .actions_for(MetadataName::FieldMetadata)
        .update
        .iter()
        .filter_map(|action| {
            let update = action.clone().into_update()?;

            update
                .update
                .get("defaultValue")
                .is_some()
                .then_some(update.universal_identifier)
        })
        .collect();

    for field_universal_identifier in touched_field_universal_identifiers {
        let Some(field) = optimistic_maps
            .field_metadata_maps
            .find_by_universal_identifier(&field_universal_identifier)
        else {
            continue;
        };

        if !field.is_enum_type() || field.default_value.is_none() {
            continue;
        }

        let default_values: Vec<String> = match field.default_value.as_ref().unwrap() {
            serde_json::Value::Array(values) => values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect(),
            serde_json::Value::String(value) => vec![value.clone()],
            _ => continue,
        };

        let option_values = option_values_by_field
            .get(&field_universal_identifier)
            .cloned()
            .unwrap_or_default();

        let default_values_without_option: Vec<String> = default_values
            .iter()
            .filter(|default_value| !option_values.contains(&unquote_default_value(default_value)))
            .cloned()
            .collect();

        let inactive_option_values =
            inactive_option_values_by_field.get(&field_universal_identifier);

        let is_default_value_set = created_field_universal_identifiers
            .contains(&field_universal_identifier)
            || field_universal_identifiers_with_default_value_update
                .contains(&field_universal_identifier);

        let from_inactive_option_values: HashSet<String> = from_flat_field_metadata_option_maps
            .by_universal_identifier
            .values()
            .filter(|option| {
                option.field_metadata_universal_identifier == field_universal_identifier
            })
            .filter(|option| !option.is_active)
            .map(|option| option.value.clone())
            .collect();

        let has_inactive_default_option = default_values
            .iter()
            .map(|default_value| unquote_default_value(default_value))
            .any(|default_option_value| {
                inactive_option_values
                    .map(|values| values.contains(&default_option_value))
                    .unwrap_or(false)
                    && (is_default_value_set
                        || !from_inactive_option_values.contains(&default_option_value))
            });

        if default_values_without_option.is_empty() && !has_inactive_default_option {
            continue;
        }

        let is_create = created_field_universal_identifiers.contains(&field_universal_identifier);

        let mut errors: Vec<FlatEntityValidationError> = Vec::new();

        for default_value in &default_values_without_option {
            errors.push(FlatEntityValidationError {
                code: "INVALID_FIELD_INPUT".to_string(),
                message: format!(
                    "Default value \"{default_value}\" must be one of the option values"
                ),
                user_friendly_message: Some(format!(
                    "Default value \"{default_value}\" must be one of the option values"
                )),
                value: Some(serde_json::Value::from(default_value.as_str())),
            });
        }

        if has_inactive_default_option {
            errors.push(FlatEntityValidationError {
                code: "INVALID_FIELD_INPUT".to_string(),
                message: "The default option cannot be deactivated".to_string(),
                user_friendly_message: Some("The default option cannot be deactivated".to_string()),
                value: field.default_value.clone(),
            });
        }

        failures.push(FailedFlatEntityValidation {
            validation_type: if is_create {
                "create".to_string()
            } else {
                "update".to_string()
            },
            metadata_name: Some(MetadataName::FieldMetadata),
            errors,
            flat_entity_minimal_information: serde_json::json!({
                "universalIdentifier": field.universal_identifier,
                "name": field.name,
                "objectMetadataUniversalIdentifier": field.object_metadata_universal_identifier
            }),
        });
    }

    failures
}

fn unquote_default_value(default_value: &str) -> String {
    let trimmed = default_value.trim();

    if trimmed.len() >= 2 && trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        return trimmed[1..trimmed.len() - 1].to_string();
    }

    trimmed.to_string()
}

pub fn compute_search_vector_rebuild_target_universal_identifiers(
    actions_report: &OrchestratorActionsReport,
    from_flat_search_field_metadata_maps: &UniversalFlatEntityMaps<
        UniversalFlatSearchFieldMetadata,
    >,
    to_flat_search_field_metadata_maps: &UniversalFlatEntityMaps<UniversalFlatSearchFieldMetadata>,
    to_flat_field_metadata_maps: &UniversalFlatEntityMaps<UniversalFlatFieldMetadata>,
) -> HashSet<String> {
    let search_field_actions = actions_report.actions_for(MetadataName::SearchFieldMetadata);

    let renamed_field_search_field_metadata_universal_identifiers: Vec<String> = actions_report
        .actions_for(MetadataName::FieldMetadata)
        .update
        .iter()
        .filter_map(|action| {
            let update = action.clone().into_update()?;

            update
                .update
                .get("name")
                .is_some()
                .then_some(update.universal_identifier)
        })
        .filter_map(|field_universal_identifier| {
            to_flat_field_metadata_maps
                .find_by_universal_identifier(&field_universal_identifier)
                .cloned()
        })
        .flat_map(|field| field.search_field_metadata_universal_identifiers)
        .collect();

    let mut candidate_field_metadata_vector_universal_identifiers: HashSet<String> = HashSet::new();

    let mut collect_vector = |candidate: Option<String>| {
        if let Some(vector) = candidate {
            candidate_field_metadata_vector_universal_identifiers.insert(vector);
        }
    };

    for create_action in &search_field_actions.create {
        let ts_vector = create_action
            .clone()
            .into_create()
            .and_then(|create| create.flat_entity)
            .and_then(|entity| entity.to_value().ok())
            .and_then(|value| {
                value
                    .get("tsVectorFieldMetadataUniversalIdentifier")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            });

        collect_vector(ts_vector);
    }

    for update_action in &search_field_actions.update {
        let ts_vector = update_action
            .clone()
            .into_update()
            .and_then(|update| {
                to_flat_search_field_metadata_maps
                    .find_by_universal_identifier(&update.universal_identifier)
                    .cloned()
            })
            .map(|search_field| search_field.ts_vector_field_metadata_universal_identifier);

        collect_vector(ts_vector);
    }

    for delete_action in &search_field_actions.delete {
        let ts_vector = delete_action
            .clone()
            .into_delete()
            .and_then(|delete| {
                from_flat_search_field_metadata_maps
                    .find_by_universal_identifier(&delete.universal_identifier)
                    .cloned()
            })
            .map(|search_field| search_field.ts_vector_field_metadata_universal_identifier);

        collect_vector(ts_vector);
    }

    for search_field_universal_identifier in
        renamed_field_search_field_metadata_universal_identifiers
    {
        let ts_vector = to_flat_search_field_metadata_maps
            .find_by_universal_identifier(&search_field_universal_identifier)
            .map(|search_field| {
                search_field
                    .ts_vector_field_metadata_universal_identifier
                    .clone()
            });

        collect_vector(ts_vector);
    }

    let mut field_universal_identifiers_being_created_or_deleted: HashSet<String> = HashSet::new();

    for create_action in &actions_report
        .actions_for(MetadataName::FieldMetadata)
        .create
    {
        if let Some(universal_identifier) = create_action.universal_identifier() {
            field_universal_identifiers_being_created_or_deleted
                .insert(universal_identifier.to_string());
        }
    }

    for create_object_action in &actions_report
        .actions_for(MetadataName::ObjectMetadata)
        .create
    {
        let Some(create) = create_object_action.clone().into_create() else {
            continue;
        };

        for field in create.universal_flat_field_metadatas {
            field_universal_identifiers_being_created_or_deleted
                .insert(field.universal_identifier.clone());
        }
    }

    for delete_action in &actions_report
        .actions_for(MetadataName::FieldMetadata)
        .delete
    {
        if let Some(universal_identifier) = delete_action.universal_identifier() {
            field_universal_identifiers_being_created_or_deleted
                .insert(universal_identifier.to_string());
        }
    }

    candidate_field_metadata_vector_universal_identifiers
        .difference(&field_universal_identifiers_being_created_or_deleted)
        .cloned()
        .collect()
}
