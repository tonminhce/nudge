#![allow(dead_code)]

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalCreateAction;
use crate::metadata::actions::UniversalDeleteAction;
use crate::metadata::actions::UniversalUpdateAction;
use crate::metadata::builders::CreationValidationContext;
use crate::metadata::builders::DeletionValidationContext;
use crate::metadata::builders::EntityActionsBuilder;
use crate::metadata::builders::FailedFlatEntityValidation;
use crate::metadata::builders::FlatEntityValidationError;
use crate::metadata::builders::UpdateValidationContext;
use crate::metadata::builders::IDENTIFIER_MAX_CHAR_LENGTH;
use crate::metadata::builders::IDENTIFIER_MIN_CHAR_LENGTH;
use crate::metadata::builders::RESERVED_METADATA_NAME_KEYWORDS;
use crate::metadata::flat::UniversalEntityLike;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalFlatObjectMetadata;
use crate::metadata::flat::UniversalWorkspaceMaps;

pub struct ObjectActionsBuilder;

pub const OBJECT_METADATA_NOT_FOUND: &str = "OBJECT_METADATA_NOT_FOUND";
pub const INVALID_OBJECT_INPUT: &str = "INVALID_OBJECT_INPUT";
pub const OBJECT_ALREADY_EXISTS: &str = "OBJECT_ALREADY_EXISTS";
pub const OBJECT_MUTATION_NOT_ALLOWED: &str = "OBJECT_MUTATION_NOT_ALLOWED";

pub const DISABLEABLE_OBJECT_PERMISSION_ACTIONS: &[&str] = &[
    "canReadObjectRecords",
    "canUpdateObjectRecords",
    "canSoftDeleteObjectRecords",
    "canDestroyObjectRecords",
    "canCreateObjectRecords",
];

impl EntityActionsBuilder for ObjectActionsBuilder {
    type Entity = UniversalFlatObjectMetadata;

    fn metadata_name(&self) -> MetadataName {
        MetadataName::ObjectMetadata
    }

    fn maps_of<'m>(
        &self,
        maps: &'m UniversalWorkspaceMaps,
    ) -> &'m UniversalFlatEntityMaps<Self::Entity> {
        &maps.object_metadata_maps
    }

    fn maps_of_mut<'m>(
        &self,
        maps: &'m mut UniversalWorkspaceMaps,
    ) -> &'m mut UniversalFlatEntityMaps<Self::Entity> {
        &mut maps.object_metadata_maps
    }

    fn validate_creation(
        &self,
        context: &CreationValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let object = &context.flat_entity_to_validate;
        let mut errors: Vec<FlatEntityValidationError> = Vec::new();

        if context
            .optimistic_maps
            .object_metadata_maps
            .by_universal_identifier
            .contains_key(&object.universal_identifier)
        {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Object with same universal identifier already exists",
                Some("Object with same universal identifier already exists"),
                None,
            ));
        }

        if object.is_remote {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Remote objects are not supported yet",
                Some("Remote objects are not supported yet"),
                None,
            ));
        }

        errors.extend(validate_object_names_and_labels(
            object,
            &context.optimistic_maps.object_metadata_maps,
            context.build_options.is_caller_twenty_standard_app(),
        ));

        errors.extend(validate_disabled_object_permission_actions(
            object.disabled_object_permission_actions.as_ref(),
        ));

        if !errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "create".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: serde_json::to_value(object.minimal_information())
                    .unwrap_or(serde_json::Value::Null),
            });
        }

        Ok(UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::ObjectMetadata,
            flat_entity: Some(
                crate::metadata::actions::UniversalFlatEntity::ObjectMetadata(Box::new(
                    object.clone(),
                )),
            ),
            universal_flat_field_metadatas: Vec::new(),
            ..UniversalCreateAction::default()
        }))
    }

    fn validate_deletion(
        &self,
        context: &DeletionValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let universal_identifier = context.flat_entity_to_validate.universal_identifier();
        let mut errors: Vec<FlatEntityValidationError> = Vec::new();
        let existing = context
            .optimistic_maps
            .object_metadata_maps
            .find_by_universal_identifier(universal_identifier);

        let Some(existing) = existing else {
            return Err(FailedFlatEntityValidation {
                validation_type: "delete".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    OBJECT_METADATA_NOT_FOUND,
                    "Object to delete not found",
                    Some("Object to delete not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        };

        let minimal =
            serde_json::to_value(existing.minimal_information()).unwrap_or(serde_json::Value::Null);

        if existing.is_remote {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Remote objects are not supported yet",
                Some("Remote objects are not supported yet"),
                None,
            ));
        }

        if !context.build_options.is_system_build && existing.is_system {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "System objects cannot be deleted",
                Some("System objects cannot be deleted"),
                None,
            ));
        }

        if !context.build_options.is_caller_twenty_standard_app()
            && existing.application_universal_identifier
                == crate::metadata::builders::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER
        {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Standard objects cannot be deleted",
                Some("Standard objects cannot be deleted"),
                None,
            ));
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
            metadata_name: MetadataName::ObjectMetadata,
            universal_identifier: universal_identifier.to_string(),
            flat_entity: None,
        }))
    }

    fn validate_update(
        &self,
        context: &UpdateValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let universal_identifier = context.universal_identifier;
        let mut errors: Vec<FlatEntityValidationError> = Vec::new();

        let Some(existing) = context
            .optimistic_maps
            .object_metadata_maps
            .find_by_universal_identifier(universal_identifier)
        else {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    OBJECT_METADATA_NOT_FOUND,
                    "Object to update not found",
                    Some("Object to update not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        };

        let mut minimal =
            serde_json::to_value(existing.minimal_information()).unwrap_or(serde_json::Value::Null);

        if let Some(minimal_object) = minimal.as_object_mut() {
            minimal_object.remove("universalIdentifier");
        }

        if !context.build_options.is_system_build && existing.is_system {
            let mut disallowed: Vec<&String> = update_keys(context.flat_entity_update)
                .filter(|property| *property != "overrides" && *property != "isActive")
                .collect();

            disallowed.sort();
            disallowed.dedup();

            if !disallowed.is_empty() {
                errors.push(error(
                    INVALID_OBJECT_INPUT,
                    "System objects cannot be updated directly. Use overrides for cosmetic changes.",
                    Some("System objects cannot be updated"),
                    None,
                ));
            }
        }

        errors.extend(validate_object_names_and_labels(
            &context.updated_flat_entity,
            &context.optimistic_maps.object_metadata_maps,
            context.build_options.is_caller_twenty_standard_app(),
        ));

        errors.extend(validate_disabled_object_permission_actions(
            context
                .updated_flat_entity
                .disabled_object_permission_actions
                .as_ref(),
        ));

        let update_object = context.flat_entity_update.as_object();

        if let Some(update_object) = update_object {
            if update_object.contains_key("labelIdentifierFieldMetadataUniversalIdentifier")
                && update_object.get("labelIdentifierFieldMetadataUniversalIdentifier")
                    == Some(&serde_json::Value::Null)
            {
                errors.push(error(
                    INVALID_OBJECT_INPUT,
                    "labelIdentifierFieldMetadataId cannot be null",
                    Some("Field label identifier is required"),
                    None,
                ));
            }
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
            metadata_name: MetadataName::ObjectMetadata,
            universal_identifier: universal_identifier.to_string(),
            update: context.flat_entity_update.clone(),
            ..UniversalUpdateAction::default()
        }))
    }
}

pub fn error<M: Into<String>, U: Into<String>>(
    code: &str,
    message: M,
    user_friendly_message: Option<U>,
    value: Option<serde_json::Value>,
) -> FlatEntityValidationError {
    FlatEntityValidationError {
        code: code.to_string(),
        message: message.into(),
        user_friendly_message: user_friendly_message.map(Into::into),
        value,
    }
}

pub fn update_keys(update: &serde_json::Value) -> impl Iterator<Item = &String> {
    update
        .as_object()
        .map(|object| object.keys())
        .into_iter()
        .flatten()
}

pub fn validate_disabled_object_permission_actions(
    disabled_actions: Option<&serde_json::Value>,
) -> Vec<FlatEntityValidationError> {
    let Some(disabled_actions) = disabled_actions else {
        return Vec::new();
    };

    if disabled_actions.is_null() {
        return Vec::new();
    }

    let Some(actions) = disabled_actions.as_array() else {
        return vec![error(
            INVALID_OBJECT_INPUT,
            "Disabled object permission actions must be a list",
            Some("Disabled object permission actions must be a list"),
            Some(disabled_actions.clone()),
        )];
    };

    let mut errors: Vec<FlatEntityValidationError> = Vec::new();

    for action in actions {
        let Some(action_name) = action.as_str() else {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                format!("Unknown disabled object permission action \"{action}\""),
                Some("Unknown disabled object permission action"),
                Some(action.clone()),
            ));

            continue;
        };

        let disableable = DISABLEABLE_OBJECT_PERMISSION_ACTIONS;

        if !disableable.contains(&action_name) {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                format!("Unknown disabled object permission action \"{action_name}\""),
                Some("Unknown disabled object permission action"),
                Some(action.clone()),
            ));
        }
    }

    let distinct: std::collections::HashSet<&serde_json::Value> = actions.iter().collect();

    if distinct.len() != actions.len() {
        errors.push(error(
            INVALID_OBJECT_INPUT,
            "Disabled object permission actions must not repeat",
            Some("Disabled object permission actions must not repeat"),
            Some(disabled_actions.clone()),
        ));
    }

    errors
}

pub fn validate_object_names_and_labels(
    object: &UniversalFlatObjectMetadata,
    object_maps: &UniversalFlatEntityMaps<UniversalFlatObjectMetadata>,
    is_standard_app_caller: bool,
) -> Vec<FlatEntityValidationError> {
    let mut errors = validate_object_names(object, is_standard_app_caller);

    for label in [&object.label_singular, &object.label_plural] {
        if label.chars().count() < IDENTIFIER_MIN_CHAR_LENGTH {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Object label is too short",
                Some("Object label is too short"),
                Some(serde_json::Value::from(label.as_str())),
            ));
        }

        if label.chars().count() > IDENTIFIER_MAX_CHAR_LENGTH {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Object label is too long",
                Some("Object label is too long"),
                Some(serde_json::Value::from(label.as_str())),
            ));
        }
    }

    if object.is_label_synced_with_name {
        let names_synced = camel_case(&object.label_singular) == object.name_singular
            && camel_case(&object.label_plural) == object.name_plural;

        if !names_synced {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Names are not synced with labels",
                Some("Names are not synced with labels"),
                None,
            ));
        }
    }

    let name_taken = object_maps.by_universal_identifier.values().any(|other| {
        (other.name_singular == object.name_singular
            || other.name_plural == object.name_plural
            || other.name_singular == object.name_plural
            || other.name_plural == object.name_singular)
            && other.universal_identifier != object.universal_identifier
    });

    if name_taken {
        errors.push(error(
            OBJECT_ALREADY_EXISTS,
            "Object already exists",
            Some("Object already exists"),
            None,
        ));
    }

    errors
}

pub fn validate_object_names(
    object: &UniversalFlatObjectMetadata,
    is_standard_app_caller: bool,
) -> Vec<FlatEntityValidationError> {
    let mut errors: Vec<FlatEntityValidationError> = Vec::new();

    for name in [&object.name_singular, &object.name_plural] {
        let length = name.chars().count();

        if length > IDENTIFIER_MAX_CHAR_LENGTH {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Name is too long",
                Some("Name is too long"),
                Some(serde_json::Value::from(name.as_str())),
            ));
        }

        if length < IDENTIFIER_MIN_CHAR_LENGTH {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Name is too short",
                Some("Name is too short"),
                Some(serde_json::Value::from(name.as_str())),
            ));
        }

        if !valid_metadata_name(name) {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "Name is not valid: it must start with lowercase letter and contain only alphanumeric letters",
                Some("Name is not valid: it must start with lowercase letter and contain only alphanumeric letters"),
                Some(serde_json::Value::from(name.as_str())),
            ));
        } else if camel_case(name) != *name {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                format!(
                    "Name is not valid: it must be camelCase, e.g. \"{}\"",
                    camel_case(name)
                ),
                Some(format!(
                    "Name is not valid: it must be camelCase, e.g. \"{}\"",
                    camel_case(name)
                )),
                Some(serde_json::Value::from(name.as_str())),
            ));
        }

        if !is_standard_app_caller && RESERVED_METADATA_NAME_KEYWORDS.contains(&name.as_str()) {
            errors.push(error(
                INVALID_OBJECT_INPUT,
                "This name is reserved. Use a different name or the system will add \"Custom\" suffix.",
                Some("This name is reserved. Use a different name or the system will add \"Custom\" suffix."),
                Some(serde_json::Value::from(name.as_str())),
            ));
        }
    }

    if object.name_plural.trim().to_lowercase() == object.name_singular.trim().to_lowercase() {
        errors.push(error(
            INVALID_OBJECT_INPUT,
            "The singular and plural names cannot be the same for an object",
            Some("The singular and plural names cannot be the same for an object"),
            Some(serde_json::Value::from(object.name_plural.as_str())),
        ));
    }

    errors
}

pub fn valid_metadata_name(name: &str) -> bool {
    let mut characters = name.chars();

    match characters.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }

    characters.all(|character| character.is_ascii_alphanumeric())
}

pub fn camel_case(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = false;
    let mut first_word = true;

    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            if first_word {
                result.push(character.to_ascii_lowercase());
            } else if capitalize_next {
                result.push(character.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(character);
            }

            first_word = false;
        } else {
            capitalize_next = true;
        }
    }

    result
}

pub fn is_upper_snake_case(value: &str) -> bool {
    let mut characters = value.chars();

    match characters.next() {
        Some(first) if first.is_ascii_uppercase() => {}
        _ => return false,
    }

    let mut previous_was_underscore = false;

    for character in characters {
        if character == '_' {
            if previous_was_underscore {
                return false;
            }

            previous_was_underscore = true;
        } else if character.is_ascii_uppercase() || character.is_ascii_digit() {
            previous_was_underscore = false;
        } else {
            return false;
        }
    }

    !previous_was_underscore
}

impl UniversalEntityLike for UniversalFlatObjectMetadata {
    fn universal_identifier(&self) -> &str {
        &self.universal_identifier
    }

    fn merge_update(&self, update: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut payload = serde_json::to_value(self)?;
        let object = payload
            .as_object_mut()
            .ok_or_else(|| serde::de::Error::custom("object entity must serialize to a map"))?;

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
