#![allow(dead_code)]

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalCreateAction;
use crate::metadata::actions::UniversalDeleteAction;
use crate::metadata::actions::UniversalUpdateAction;
use crate::metadata::builders::object::error;
use crate::metadata::builders::object::is_upper_snake_case;
use crate::metadata::builders::CreationValidationContext;
use crate::metadata::builders::DeletionValidationContext;
use crate::metadata::builders::EntityActionsBuilder;
use crate::metadata::builders::FailedFlatEntityValidation;
use crate::metadata::builders::UpdateValidationContext;
use crate::metadata::builders::IDENTIFIER_MAX_CHAR_LENGTH;
use crate::metadata::builders::IDENTIFIER_MIN_CHAR_LENGTH;
use crate::metadata::builders::TAG_COLORS;
use crate::metadata::flat::UniversalEntityLike;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalFlatFieldMetadataOption;
use crate::metadata::flat::UniversalWorkspaceMaps;

pub struct FieldMetadataOptionActionsBuilder;

pub const FIELD_METADATA_OPTION_NOT_FOUND: &str = "FIELD_METADATA_OPTION_NOT_FOUND";
pub const INVALID_FIELD_METADATA_OPTION_INPUT: &str = "INVALID_FIELD_METADATA_OPTION_INPUT";

impl EntityActionsBuilder for FieldMetadataOptionActionsBuilder {
    type Entity = UniversalFlatFieldMetadataOption;

    fn metadata_name(&self) -> MetadataName {
        MetadataName::FieldMetadataOption
    }

    fn maps_of<'m>(
        &self,
        maps: &'m UniversalWorkspaceMaps,
    ) -> &'m UniversalFlatEntityMaps<Self::Entity> {
        &maps.field_metadata_option_maps
    }

    fn maps_of_mut<'m>(
        &self,
        maps: &'m mut UniversalWorkspaceMaps,
    ) -> &'m mut UniversalFlatEntityMaps<Self::Entity> {
        &mut maps.field_metadata_option_maps
    }

    fn validate_creation(
        &self,
        context: &CreationValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let option = &context.flat_entity_to_validate;
        let errors = collect_option_errors(
            option,
            &context.optimistic_maps.field_metadata_maps,
            &context.final_maps.by_universal_identifier,
        );

        if !errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "create".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": option.universal_identifier
                }),
            });
        }

        Ok(UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::FieldMetadataOption,
            flat_entity: Some(
                crate::metadata::actions::UniversalFlatEntity::FieldMetadataOption(Box::new(
                    option.clone(),
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
        let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

        let existing = context
            .optimistic_maps
            .field_metadata_option_maps
            .find_by_universal_identifier(universal_identifier);

        let Some(existing) = existing else {
            return Err(FailedFlatEntityValidation {
                validation_type: "delete".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    FIELD_METADATA_OPTION_NOT_FOUND,
                    "Field option not found",
                    Some("Field option not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        };

        let minimal = serde_json::json!({
            "universalIdentifier": existing.universal_identifier
        });

        if !caller_allowed_to_remove_or_revalue(
            existing,
            &context.optimistic_maps.field_metadata_maps,
            context.build_options.is_system_build,
            &context.build_options.application_universal_identifier,
        ) {
            errors.push(error(
                INVALID_FIELD_METADATA_OPTION_INPUT,
                format!(
                    "Option \"{}\" comes with the field and cannot be removed. Deactivate it instead.",
                    existing.label
                ),
                Some(format!(
                    "Option \"{}\" comes with the field and cannot be removed. Deactivate it instead.",
                    existing.label
                )),
                Some(serde_json::Value::from(existing.value.as_str())),
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
            metadata_name: MetadataName::FieldMetadataOption,
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
            .field_metadata_option_maps
            .find_by_universal_identifier(universal_identifier)
        else {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    FIELD_METADATA_OPTION_NOT_FOUND,
                    "Field option not found",
                    Some("Field option not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        };

        let minimal = serde_json::json!({
            "universalIdentifier": existing.universal_identifier
        });

        let requested_value = context
            .flat_entity_update
            .get("value")
            .and_then(serde_json::Value::as_str);

        if let Some(requested_value) = requested_value {
            if requested_value != existing.value
                && !caller_allowed_to_remove_or_revalue(
                    existing,
                    &context.optimistic_maps.field_metadata_maps,
                    context.build_options.is_system_build,
                    &context.build_options.application_universal_identifier,
                )
            {
                errors.push(error(
                    INVALID_FIELD_METADATA_OPTION_INPUT,
                    format!(
                        "The API value of option \"{}\" comes with the field and cannot be changed.",
                        existing.label
                    ),
                    Some(format!(
                        "The API value of option \"{}\" comes with the field and cannot be changed.",
                        existing.label
                    )),
                    Some(serde_json::Value::from(existing.value.as_str())),
                ));
            }
        }

        let merged_errors = collect_option_errors(
            &context.updated_flat_entity,
            &context.optimistic_maps.field_metadata_maps,
            &context
                .optimistic_maps
                .field_metadata_option_maps
                .by_universal_identifier,
        );

        errors.extend(merged_errors);

        if !errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: minimal,
            });
        }

        Ok(UniversalAction::Update(UniversalUpdateAction {
            metadata_name: MetadataName::FieldMetadataOption,
            universal_identifier: universal_identifier.to_string(),
            update: context.flat_entity_update.clone(),
            ..UniversalUpdateAction::default()
        }))
    }
}

fn caller_allowed_to_remove_or_revalue(
    option: &UniversalFlatFieldMetadataOption,
    field_maps: &UniversalFlatEntityMaps<crate::metadata::flat::UniversalFlatFieldMetadata>,
    is_system_build: bool,
    caller_application_universal_identifier: &str,
) -> bool {
    if is_system_build
        || option.application_universal_identifier == caller_application_universal_identifier
    {
        return true;
    }

    let owning_field =
        field_maps.find_by_universal_identifier(&option.field_metadata_universal_identifier);

    let Some(owning_field) = owning_field else {
        return true;
    };

    owning_field.application_universal_identifier == caller_application_universal_identifier
}

fn collect_option_errors(
    option: &UniversalFlatFieldMetadataOption,
    field_maps: &UniversalFlatEntityMaps<crate::metadata::flat::UniversalFlatFieldMetadata>,
    final_options_by_universal_identifier: &std::collections::HashMap<
        String,
        UniversalFlatFieldMetadataOption,
    >,
) -> Vec<crate::metadata::builders::FlatEntityValidationError> {
    let Some(field) =
        field_maps.find_by_universal_identifier(&option.field_metadata_universal_identifier)
    else {
        return vec![error(
            INVALID_FIELD_METADATA_OPTION_INPUT,
            "Field option field not found",
            Some("Field option field not found"),
            None,
        )];
    };

    if !field.is_enum_type() {
        return vec![error(
            INVALID_FIELD_METADATA_OPTION_INPUT,
            "Options can only belong to select, multi-select or rating fields",
            Some("Options can only belong to select, multi-select or rating fields"),
            None,
        )];
    }

    let mut errors = validate_metadata_option_value(&option.value);
    errors.extend(validate_metadata_option_label(&option.label));
    errors.extend(validate_metadata_option_color(
        &option.label,
        option.color.as_deref(),
    ));

    let has_duplicate_value = final_options_by_universal_identifier.values().any(|other| {
        other.universal_identifier != option.universal_identifier
            && other.field_metadata_universal_identifier
                == option.field_metadata_universal_identifier
            && other.value == option.value
    });

    if has_duplicate_value {
        errors.push(error(
            INVALID_FIELD_METADATA_OPTION_INPUT,
            format!(
                "Option value \"{}\" already exists on this field",
                option.value
            ),
            Some(format!(
                "Option value \"{}\" already exists on this field",
                option.value
            )),
            Some(serde_json::Value::from(option.value.as_str())),
        ));
    }

    errors
}

pub fn validate_metadata_option_value(
    value: &str,
) -> Vec<crate::metadata::builders::FlatEntityValidationError> {
    let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

    if value.is_empty() {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            "Option value must be a string of at least one character",
            Some("Option value format not supported"),
            Some(serde_json::Value::from(value)),
        ));

        return errors;
    }

    if value.chars().count() > IDENTIFIER_MAX_CHAR_LENGTH {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            "Option value exceeds 63 characters",
            Some("Option value exceeds 63 characters"),
            Some(serde_json::Value::from(value)),
        ));
    }

    if value.chars().count() < IDENTIFIER_MIN_CHAR_LENGTH {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            format!("Option value \"{value}\" is beneath 1 character"),
            Some(format!("Option value \"{value}\" is beneath 1 character")),
            Some(serde_json::Value::from(value)),
        ));
    }

    if !is_upper_snake_case(value) {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            format!("Value must be in UPPER_CASE and follow snake_case \"{value}\""),
            Some(format!(
                "Value must be in UPPER_CASE and follow snake_case \"{value}\""
            )),
            Some(serde_json::Value::from(value)),
        ));
    }

    errors
}

pub fn validate_metadata_option_label(
    label: &str,
) -> Vec<crate::metadata::builders::FlatEntityValidationError> {
    let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

    if label.is_empty() {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            "Option label must be a string of at least one character",
            Some("Option label format not supported"),
            Some(serde_json::Value::from(label)),
        ));

        return errors;
    }

    if label.chars().count() > IDENTIFIER_MAX_CHAR_LENGTH {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            "Option label exceeds 63 characters",
            Some("Option label exceeds 63 characters"),
            Some(serde_json::Value::from(label)),
        ));
    }

    if label.chars().count() < IDENTIFIER_MIN_CHAR_LENGTH {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            format!("Option label \"{label}\" is beneath 1 character"),
            Some(format!("Option label \"{label}\" is beneath 1 character")),
            Some(serde_json::Value::from(label)),
        ));
    }

    if label.contains(',') {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            "Label must not contain a comma",
            Some("Label must not contain a comma"),
            Some(serde_json::Value::from(label)),
        ));
    }

    if label == " " {
        errors.push(error(
            crate::metadata::builders::field::INVALID_FIELD_INPUT,
            "Label must not be empty",
            Some("Label must not be empty"),
            Some(serde_json::Value::from(label)),
        ));
    }

    errors
}

pub fn validate_metadata_option_color(
    option_label: &str,
    color: Option<&str>,
) -> Vec<crate::metadata::builders::FlatEntityValidationError> {
    let Some(color) = color else {
        return Vec::new();
    };

    if TAG_COLORS.contains(&color) {
        return Vec::new();
    }

    vec![error(
        crate::metadata::builders::field::INVALID_FIELD_INPUT,
        format!(
            "Option \"{option_label}\" color \"{color}\" is not supported. Supported colors: {}",
            TAG_COLORS.join(", ")
        ),
        Some(format!(
            "Option \"{option_label}\" has an unsupported color"
        )),
        Some(serde_json::Value::from(color)),
    )]
}

impl UniversalEntityLike for UniversalFlatFieldMetadataOption {
    fn universal_identifier(&self) -> &str {
        &self.universal_identifier
    }

    fn merge_update(&self, update: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut payload = serde_json::to_value(self)?;
        let object = payload
            .as_object_mut()
            .ok_or_else(|| serde::de::Error::custom("option entity must serialize to a map"))?;

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
