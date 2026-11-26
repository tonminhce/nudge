#![allow(dead_code)]

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalCreateAction;
use crate::metadata::actions::UniversalDeleteAction;
use crate::metadata::actions::UniversalUpdateAction;
use crate::metadata::builders::object::error;
use crate::metadata::builders::CreationValidationContext;
use crate::metadata::builders::DeletionValidationContext;
use crate::metadata::builders::EntityActionsBuilder;
use crate::metadata::builders::FailedFlatEntityValidation;
use crate::metadata::builders::UpdateValidationContext;
use crate::metadata::flat::RelationType;
use crate::metadata::flat::UniversalEntityLike;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalFlatIndexMetadata;
use crate::metadata::flat::UniversalWorkspaceMaps;

pub struct IndexActionsBuilder;

pub const INDEX_EMPTY_FIELDS: &str = "INDEX_EMPTY_FIELDS";
pub const INDEX_FIELD_NOT_FOUND: &str = "INDEX_FIELD_NOT_FOUND";
pub const INDEX_FIELD_WRONG_OBJECT: &str = "INDEX_FIELD_WRONG_OBJECT";
pub const INDEX_FIELD_INVALID_REFERENCE: &str = "INDEX_FIELD_INVALID_REFERENCE";
pub const INDEX_ALREADY_EXISTS: &str = "INDEX_ALREADY_EXISTS";
pub const INDEX_NOT_FOUND: &str = "INDEX_NOT_FOUND";
pub const INDEX_OBJECT_NOT_FOUND: &str = "INDEX_OBJECT_NOT_FOUND";
pub const INDEX_FIELD_INVALID_TYPE_FOR_UNIQUE: &str = "INDEX_FIELD_INVALID_TYPE_FOR_UNIQUE";
pub const INDEX_FIELD_INVALID_DEFAULT_VALUE: &str = "INDEX_FIELD_INVALID_DEFAULT_VALUE";

impl EntityActionsBuilder for IndexActionsBuilder {
    type Entity = UniversalFlatIndexMetadata;

    fn metadata_name(&self) -> MetadataName {
        MetadataName::Index
    }

    fn maps_of<'m>(
        &self,
        maps: &'m UniversalWorkspaceMaps,
    ) -> &'m UniversalFlatEntityMaps<Self::Entity> {
        &maps.index_metadata_maps
    }

    fn maps_of_mut<'m>(
        &self,
        maps: &'m mut UniversalWorkspaceMaps,
    ) -> &'m mut UniversalFlatEntityMaps<Self::Entity> {
        &mut maps.index_metadata_maps
    }

    fn validate_creation(
        &self,
        context: &CreationValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let errors = validate_index_creation(
            &context.flat_entity_to_validate,
            &context.optimistic_maps.index_metadata_maps,
            &context.optimistic_maps.object_metadata_maps,
            &context.optimistic_maps.field_metadata_maps,
        );

        if !errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "create".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: serde_json::to_value(
                    context.flat_entity_to_validate.minimal_information(),
                )
                .unwrap_or(serde_json::Value::Null),
            });
        }

        Ok(UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::Index,
            flat_entity: Some(
                crate::metadata::actions::UniversalFlatEntity::IndexMetadata(Box::new(
                    context.flat_entity_to_validate.clone(),
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

        if context
            .optimistic_maps
            .index_metadata_maps
            .find_by_universal_identifier(universal_identifier)
            .is_none()
        {
            return Err(FailedFlatEntityValidation {
                validation_type: "delete".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    INDEX_NOT_FOUND,
                    "Index to delete not found",
                    None::<String>,
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        }

        Ok(UniversalAction::Delete(UniversalDeleteAction {
            metadata_name: MetadataName::Index,
            universal_identifier: universal_identifier.to_string(),
            flat_entity: None,
        }))
    }

    fn validate_update(
        &self,
        context: &UpdateValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let universal_identifier = context.universal_identifier;

        let Some(existing) = context
            .optimistic_maps
            .index_metadata_maps
            .find_by_universal_identifier(universal_identifier)
        else {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    INDEX_NOT_FOUND,
                    "Index to delete not found",
                    None::<String>,
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        };

        let mut temporary_index_maps = context.optimistic_maps.index_metadata_maps.clone();

        temporary_index_maps.delete_by_universal_identifier(existing.universal_identifier());

        let errors = validate_index_creation(
            &context.updated_flat_entity,
            &temporary_index_maps,
            &context.optimistic_maps.object_metadata_maps,
            &context.optimistic_maps.field_metadata_maps,
        );

        if !errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        }

        Ok(UniversalAction::Update(UniversalUpdateAction {
            metadata_name: MetadataName::Index,
            universal_identifier: universal_identifier.to_string(),
            update: context.flat_entity_update.clone(),
            updated_universal_flat_index: Some(context.updated_flat_entity.clone()),
            ..UniversalUpdateAction::default()
        }))
    }
}

fn validate_index_creation(
    index: &UniversalFlatIndexMetadata,
    index_maps: &UniversalFlatEntityMaps<UniversalFlatIndexMetadata>,
    object_maps: &UniversalFlatEntityMaps<crate::metadata::flat::UniversalFlatObjectMetadata>,
    field_maps: &UniversalFlatEntityMaps<crate::metadata::flat::UniversalFlatFieldMetadata>,
) -> Vec<crate::metadata::builders::FlatEntityValidationError> {
    let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

    if index_maps
        .by_universal_identifier
        .contains_key(&index.universal_identifier)
    {
        errors.push(error(
            INDEX_ALREADY_EXISTS,
            "Index with same universal identifier already exists",
            Some("Index already exists"),
            None,
        ));
    }

    let related_object =
        object_maps.find_by_universal_identifier(&index.object_metadata_universal_identifier);

    if related_object.is_none() {
        errors.push(error(
            INDEX_OBJECT_NOT_FOUND,
            "Could not find index related object metadata",
            Some("Index related object not found"),
            None,
        ));
    }

    let name_taken = index_maps
        .by_universal_identifier
        .values()
        .any(|existing| existing.name.to_uppercase() == index.name.to_uppercase());

    if name_taken {
        errors.push(error(
            INDEX_ALREADY_EXISTS,
            "Index with same name already exists",
            Some("Index with same name already exists"),
            None,
        ));
    }

    if index.universal_flat_index_field_metadatas.is_empty() {
        errors.push(error(
            INDEX_EMPTY_FIELDS,
            "Index must have at least one field",
            Some("An index must contain at least one field"),
            None,
        ));
    } else {
        for index_field in &index.universal_flat_index_field_metadatas {
            let related_field = field_maps
                .find_by_universal_identifier(&index_field.field_metadata_universal_identifier);

            match related_field {
                None => {
                    errors.push(error(
                        INDEX_FIELD_NOT_FOUND,
                        "Could not find index field related field metadata",
                        Some("Field referenced in index does not exist"),
                        None,
                    ));
                }
                Some(field) => {
                    if field.object_metadata_universal_identifier
                        != index.object_metadata_universal_identifier
                    {
                        errors.push(error(
                            INDEX_FIELD_WRONG_OBJECT,
                            "Field does not belong to the indexed object",
                            Some("Field cannot be indexed as it belongs to a different object"),
                            None,
                        ));
                    }

                    if field.is_morph_relation_head() {
                        errors.push(error(
                            INDEX_FIELD_INVALID_REFERENCE,
                            "Cannot index a logical morph field without a physical column",
                            Some("This field has no database column to index."),
                            None,
                        ));
                    }

                    if index.is_unique {
                        let has_default = field
                            .default_value
                            .as_ref()
                            .is_some_and(|value| !value.is_null());

                        if has_default && field.is_unique {
                            errors.push(error(
                                INDEX_FIELD_INVALID_DEFAULT_VALUE,
                                format!(
                                    "Unique index cannot be created for field {} of type {}",
                                    field.name,
                                    field.field_type.as_str()
                                ),
                                Some(format!(
                                    "{} fields cannot have a default value.",
                                    field.field_type.as_str()
                                )),
                                None,
                            ));
                        }

                        let relation_without_join_column = field.is_relation_or_morph()
                            && field
                                .universal_settings
                                .as_ref()
                                .and_then(|settings| settings.get("relationType"))
                                .and_then(serde_json::Value::as_str)
                                .and_then(RelationType::parse)
                                != Some(RelationType::ManyToOne);

                        if relation_without_join_column {
                            errors.push(error(
                                INDEX_FIELD_INVALID_TYPE_FOR_UNIQUE,
                                format!(
                                    "Unique index cannot be created for field {} of type {}",
                                    field.name,
                                    field.field_type.as_str()
                                ),
                                Some(format!(
                                    "{} fields cannot be unique.",
                                    field.field_type.as_str()
                                )),
                                None,
                            ));
                        }
                    }
                }
            }

            if index_field.index_metadata_universal_identifier != index.universal_identifier {
                errors.push(error(
                    INDEX_FIELD_INVALID_REFERENCE,
                    "Index field references incorrect index metadata",
                    Some("Index field has an invalid reference"),
                    None,
                ));
            }
        }
    }

    errors
}

impl UniversalEntityLike for UniversalFlatIndexMetadata {
    fn universal_identifier(&self) -> &str {
        &self.universal_identifier
    }

    fn merge_update(&self, update: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut payload = serde_json::to_value(self)?;
        let object = payload
            .as_object_mut()
            .ok_or_else(|| serde::de::Error::custom("index entity must serialize to a map"))?;

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
