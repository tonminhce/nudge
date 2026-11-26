#![allow(dead_code)]

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalCreateAction;
use crate::metadata::actions::UniversalDeleteAction;
use crate::metadata::actions::UniversalUpdateAction;
use crate::metadata::builders::object::error;
use crate::metadata::builders::object::update_keys;
use crate::metadata::builders::object::valid_metadata_name;
use crate::metadata::builders::CreationValidationContext;
use crate::metadata::builders::DeletionValidationContext;
use crate::metadata::builders::EntityActionsBuilder;
use crate::metadata::builders::FailedFlatEntityValidation;
use crate::metadata::builders::UpdateValidationContext;
use crate::metadata::builders::IDENTIFIER_MAX_CHAR_LENGTH;
use crate::metadata::builders::IDENTIFIER_MIN_CHAR_LENGTH;
use crate::metadata::builders::RESERVED_METADATA_NAME_KEYWORDS;
use crate::metadata::flat::UniversalEntityLike;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalFlatFieldMetadata;
use crate::metadata::flat::UniversalFlatObjectMetadata;
use crate::metadata::flat::UniversalWorkspaceMaps;

pub struct FieldActionsBuilder;

pub const FIELD_METADATA_NOT_FOUND: &str = "FIELD_METADATA_NOT_FOUND";
pub const INVALID_FIELD_INPUT: &str = "INVALID_FIELD_INPUT";
pub const FIELD_MUTATION_NOT_ALLOWED: &str = "FIELD_MUTATION_NOT_ALLOWED";
pub const FIELD_ALREADY_EXISTS: &str = "FIELD_ALREADY_EXISTS";
pub const LABEL_IDENTIFIER_FIELD_METADATA_ID_NOT_FOUND: &str =
    "LABEL_IDENTIFIER_FIELD_METADATA_ID_NOT_FOUND";
pub const NAME_NOT_SYNCED_WITH_LABEL: &str = "NAME_NOT_SYNCED_WITH_LABEL";
pub const RELATION_UNIVERSAL_IDENTIFIER_NOT_FOUND: &str = "RELATION_UNIVERSAL_IDENTIFIER_NOT_FOUND";
pub const ENTITY_ALREADY_EXISTS: &str = "ENTITY_ALREADY_EXISTS";
pub const ENTITY_NOT_FOUND: &str = "ENTITY_NOT_FOUND";
pub const ENTITY_MALFORMED: &str = "ENTITY_MALFORMED";

pub const SYSTEM_FIELD_ALLOWED_UPDATE_PROPERTIES: &[&str] = &["universalSettings", "isActive"];

pub const RELATION_EDITABLE_PROPERTIES: &[&str] = &[
    "label",
    "description",
    "icon",
    "isActive",
    "isLabelSyncedWithName",
    "isSearchable",
    "isAuditLogged",
    "overrides",
    "universalSettings",
    "isUIEditable",
    "disabledFieldPermissionActions",
];

impl EntityActionsBuilder for FieldActionsBuilder {
    type Entity = UniversalFlatFieldMetadata;

    fn metadata_name(&self) -> MetadataName {
        MetadataName::FieldMetadata
    }

    fn maps_of<'m>(
        &self,
        maps: &'m UniversalWorkspaceMaps,
    ) -> &'m UniversalFlatEntityMaps<Self::Entity> {
        &maps.field_metadata_maps
    }

    fn maps_of_mut<'m>(
        &self,
        maps: &'m mut UniversalWorkspaceMaps,
    ) -> &'m mut UniversalFlatEntityMaps<Self::Entity> {
        &mut maps.field_metadata_maps
    }

    fn validate_creation(
        &self,
        context: &CreationValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let field = &context.flat_entity_to_validate;
        let mut validation_errors: Vec<crate::metadata::builders::FlatEntityValidationError> =
            Vec::new();

        let parent_object = context
            .optimistic_maps
            .object_metadata_maps
            .find_by_universal_identifier(&field.object_metadata_universal_identifier);

        match parent_object {
            None => {
                validation_errors.push(error(
                    crate::metadata::builders::object::OBJECT_METADATA_NOT_FOUND,
                    "Object metadata not found",
                    Some("Field to create related object not found"),
                    None,
                ));
            }
            Some(parent) => {
                if context
                    .optimistic_maps
                    .field_metadata_maps
                    .by_universal_identifier
                    .contains_key(&field.universal_identifier)
                {
                    validation_errors.push(error(
                        FIELD_ALREADY_EXISTS,
                        "Field with same universal identifier already exists in object",
                        Some("Field already exists"),
                        None,
                    ));
                }

                if parent.is_remote {
                    validation_errors.push(error(
                        FIELD_MUTATION_NOT_ALLOWED,
                        "Remote objects are read-only",
                        Some("Remote objects are not production ready yet"),
                        None,
                    ));
                }

                validation_errors.extend(validate_field_name_availability(
                    &field.name,
                    parent,
                    &context.optimistic_maps.field_metadata_maps,
                ));

                if field.is_searchable
                    && !context.build_options.is_system_build
                    && parent
                        .label_identifier_field_metadata_universal_identifier
                        .as_deref()
                        != Some(field.universal_identifier.as_str())
                {
                    validation_errors.push(error(
                        INVALID_FIELD_INPUT,
                        "Searchable fields are managed by the platform",
                        Some("Searchable fields are managed by the platform"),
                        None,
                    ));
                }
            }
        }

        if field.is_label_synced_with_name && camel_case(&field.label) != field.name {
            validation_errors.push(error(
                NAME_NOT_SYNCED_WITH_LABEL,
                "Name is not synced with label",
                Some("Field name is not synced with field label"),
                Some(serde_json::Value::from(field.label.as_str())),
            ));
        }

        validation_errors.extend(validate_field_name(
            &field.name,
            context.build_options.is_caller_twenty_standard_app(),
        ));

        if field.is_enum_type() && field_option_values(field.options.as_ref()).is_empty() {
            validation_errors.push(error(
                INVALID_FIELD_INPUT,
                "Enum fields require at least one option",
                Some("Enum fields require at least one option"),
                None,
            ));
        }

        if !validation_errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "create".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: validation_errors,
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": field.universal_identifier,
                    "name": field.name,
                    "objectMetadataUniversalIdentifier":
                        field.object_metadata_universal_identifier
                }),
            });
        }

        Ok(UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::FieldMetadata,
            flat_entity: Some(
                crate::metadata::actions::UniversalFlatEntity::FieldMetadata(Box::new(
                    field.clone(),
                )),
            ),
            ..UniversalCreateAction::default()
        }))
    }

    fn validate_deletion(
        &self,
        context: &DeletionValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let universal_identifier = context.flat_entity_to_validate.universal_identifier();

        let Some(field) = context
            .optimistic_maps
            .field_metadata_maps
            .find_by_universal_identifier(universal_identifier)
        else {
            return Err(FailedFlatEntityValidation {
                validation_type: "delete".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    FIELD_METADATA_NOT_FOUND,
                    "Field metadata to delete not found",
                    Some("Field metadata to delete not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        };

        let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

        let minimal = serde_json::json!({
            "universalIdentifier": field.universal_identifier,
            "name": field.name,
            "objectMetadataUniversalIdentifier": field.object_metadata_universal_identifier
        });

        let parent_object = context
            .optimistic_maps
            .object_metadata_maps
            .find_by_universal_identifier(&field.object_metadata_universal_identifier);

        let parent_object_metadata_has_been_deleted = parent_object.is_none();

        if !context.build_options.is_system_build
            && field.is_morph_relation_head()
            && !parent_object_metadata_has_been_deleted
        {
            let has_remaining_targets = parent_object
                .map(|parent| {
                    parent
                        .field_universal_identifiers
                        .iter()
                        .any(|field_universal_identifier| {
                            field_universal_identifier.as_str()
                                != field.universal_identifier.as_str()
                                && context
                                    .remaining_maps
                                    .by_universal_identifier
                                    .get(field_universal_identifier)
                                    .map(|remaining| remaining.morph_id == field.morph_id)
                                    .unwrap_or(false)
                        })
                })
                .unwrap_or(false);

            if has_remaining_targets {
                errors.push(error(
                    FIELD_MUTATION_NOT_ALLOWED,
                    "Delete the morph field together with its targets",
                    Some("Delete the relation field together with its targets"),
                    None,
                ));
            }
        }

        if !context.build_options.is_system_build
            && field.is_system
            && !parent_object_metadata_has_been_deleted
        {
            errors.push(error(
                FIELD_MUTATION_NOT_ALLOWED,
                "System fields cannot be deleted",
                Some("System fields cannot be deleted"),
                None,
            ));
        }

        if let Some(parent) = parent_object {
            if parent
                .label_identifier_field_metadata_universal_identifier
                .as_deref()
                == Some(field.universal_identifier.as_str())
            {
                errors.push(error(
                    FIELD_MUTATION_NOT_ALLOWED,
                    "Cannot delete, please update the label identifier field first",
                    Some("Cannot delete, please update the label identifier field first"),
                    None,
                ));
            }
        }

        if !errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "delete".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: minimal,
            });
        }

        Ok(UniversalAction::Delete(UniversalDeleteAction {
            metadata_name: MetadataName::FieldMetadata,
            universal_identifier: universal_identifier.to_string(),
            flat_entity: None,
        }))
    }

    fn validate_update(
        &self,
        context: &UpdateValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let universal_identifier = context.universal_identifier;
        let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

        let Some(existing) = context
            .optimistic_maps
            .field_metadata_maps
            .find_by_universal_identifier(universal_identifier)
        else {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    FIELD_METADATA_NOT_FOUND,
                    "field metadata to update not found",
                    Some("Field to update not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        };

        let minimal = serde_json::json!({
            "universalIdentifier": existing.universal_identifier,
            "name": context.updated_flat_entity.name,
            "objectMetadataUniversalIdentifier":
                context.updated_flat_entity.object_metadata_universal_identifier
        });

        if !context.build_options.is_system_build && existing.is_system {
            let disallowed: Vec<&String> = update_keys(context.flat_entity_update)
                .filter(|property| {
                    !SYSTEM_FIELD_ALLOWED_UPDATE_PROPERTIES.contains(&property.as_str())
                })
                .collect();

            if !disallowed.is_empty() {
                let forbidden = disallowed
                    .iter()
                    .map(|property| property.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ");

                errors.push(error(
                    FIELD_MUTATION_NOT_ALLOWED,
                    format!(
                        "System fields only allow updating: {}. Forbidden properties: {}",
                        SYSTEM_FIELD_ALLOWED_UPDATE_PROPERTIES.join(", "),
                        forbidden
                    ),
                    Some("System fields cannot be updated"),
                    None,
                ));
            }
        }

        let parent_object = context
            .optimistic_maps
            .object_metadata_maps
            .find_by_universal_identifier(
                &context
                    .updated_flat_entity
                    .object_metadata_universal_identifier,
            );

        let Some(parent) = parent_object else {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    crate::metadata::builders::object::OBJECT_METADATA_NOT_FOUND,
                    "field metadata to update object metadata not found",
                    Some("Object related to field to update not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        };

        {
            if parent
                .label_identifier_field_metadata_universal_identifier
                .is_none()
            {
                errors.push(error(
                    LABEL_IDENTIFIER_FIELD_METADATA_ID_NOT_FOUND,
                    "Label identifier field metadata id does not exist",
                    Some("Object related to updated field does not have a label identifier"),
                    None,
                ));
            } else if parent
                .label_identifier_field_metadata_universal_identifier
                .as_deref()
                == Some(context.updated_flat_entity.universal_identifier.as_str())
                && (update_keys(context.flat_entity_update).any(|property| property == "isActive"))
                && !context.updated_flat_entity.is_active
            {
                errors.push(error(
                    LABEL_IDENTIFIER_FIELD_METADATA_ID_NOT_FOUND,
                    "Label identifier field metadata cannot be deactivated",
                    Some("Label identifier field cannot be deactivated"),
                    None,
                ));
            }
        }

        if context.updated_flat_entity.is_relation_or_morph() {
            let forbidden: Vec<&String> = update_keys(context.flat_entity_update)
                .filter(|property| !RELATION_EDITABLE_PROPERTIES.contains(&property.as_str()))
                .collect();

            if !forbidden.is_empty() {
                let names = forbidden
                    .iter()
                    .map(|property| property.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ");

                errors.push(error(
                    FIELD_MUTATION_NOT_ALLOWED,
                    format!("Forbidden updated properties for relation field metadata: {names}"),
                    Some("Forbidden updated properties for relation field metadata"),
                    None,
                ));
            }
        }

        if update_keys(context.flat_entity_update).any(|property| property == "name") {
            errors.extend(validate_field_name(
                &context.updated_flat_entity.name,
                context.build_options.is_caller_twenty_standard_app(),
            ));

            if let Some(parent) = parent_object {
                errors.extend(validate_field_name_availability(
                    &context.updated_flat_entity.name,
                    parent,
                    &context.optimistic_maps.field_metadata_maps,
                ));
            }
        }

        if context.updated_flat_entity.is_label_synced_with_name
            && camel_case(&context.updated_flat_entity.label) != context.updated_flat_entity.name
        {
            errors.push(error(
                FIELD_MUTATION_NOT_ALLOWED,
                "Name is not synced with label.",
                Some("Updated field name is not synced with label"),
                None,
            ));
        }

        if !errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: minimal,
            });
        }

        Ok(UniversalAction::Update(UniversalUpdateAction {
            metadata_name: MetadataName::FieldMetadata,
            universal_identifier: universal_identifier.to_string(),
            update: context.flat_entity_update.clone(),
            ..UniversalUpdateAction::default()
        }))
    }
}

pub fn camel_case(text: &str) -> String {
    crate::metadata::builders::object::camel_case(text)
}

pub fn validate_field_name(
    name: &str,
    is_standard_app_caller: bool,
) -> Vec<crate::metadata::builders::FlatEntityValidationError> {
    let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

    if name.chars().count() > IDENTIFIER_MAX_CHAR_LENGTH {
        errors.push(error(
            INVALID_FIELD_INPUT,
            "Name is too long",
            Some("Name is too long"),
            Some(serde_json::Value::from(name)),
        ));
    }

    if name.chars().count() < IDENTIFIER_MIN_CHAR_LENGTH {
        errors.push(error(
            INVALID_FIELD_INPUT,
            "Name is too short",
            Some("Name is too short"),
            Some(serde_json::Value::from(name)),
        ));
    }

    if !valid_metadata_name(name) {
        errors.push(error(
            INVALID_FIELD_INPUT,
            "Name is not valid: it must start with lowercase letter and contain only alphanumeric letters",
            Some("Name is not valid: it must start with lowercase letter and contain only alphanumeric letters"),
            Some(serde_json::Value::from(name)),
        ));
    }

    if !is_standard_app_caller && RESERVED_METADATA_NAME_KEYWORDS.contains(&name) {
        errors.push(error(
            INVALID_FIELD_INPUT,
            "This name is reserved. Use a different name or the system will add \"Custom\" suffix.",
            Some("This name is reserved. Use a different name or the system will add \"Custom\" suffix."),
            Some(serde_json::Value::from(name)),
        ));
    }

    errors
}

pub fn validate_field_name_availability(
    name: &str,
    parent: &UniversalFlatObjectMetadata,
    field_maps: &UniversalFlatEntityMaps<UniversalFlatFieldMetadata>,
) -> Vec<crate::metadata::builders::FlatEntityValidationError> {
    let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

    let name_taken = parent
        .field_universal_identifiers
        .iter()
        .filter_map(|field_universal_identifier| {
            field_maps.find_by_universal_identifier(field_universal_identifier)
        })
        .any(|other| other.name == name);

    if name_taken {
        errors.push(error(
            "NOT_AVAILABLE",
            format!("Name \"{name}\" is not available as it is already used by another field"),
            Some(format!(
                "Name \"{name}\" is not available as it is already used by another field"
            )),
            Some(serde_json::Value::from(name)),
        ));
    }

    errors
}

pub fn field_option_values(options: Option<&serde_json::Value>) -> Vec<String> {
    let Some(options) = options else {
        return Vec::new();
    };

    let Some(array) = options.as_array() else {
        return Vec::new();
    };

    array
        .iter()
        .filter_map(|option| option.get("value").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect()
}

impl UniversalEntityLike for UniversalFlatFieldMetadata {
    fn universal_identifier(&self) -> &str {
        &self.universal_identifier
    }

    fn merge_update(&self, update: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut payload = serde_json::to_value(self)?;
        let object = payload
            .as_object_mut()
            .ok_or_else(|| serde::de::Error::custom("field entity must serialize to a map"))?;

        if let Some(update_object) = update.as_object() {
            for (property, value) in update_object {
                object.insert(property.clone(), value.clone());
            }
        }

        serde_json::from_value(payload)
    }

    fn to_diff_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}
