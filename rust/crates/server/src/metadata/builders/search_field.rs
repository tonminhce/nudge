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
use crate::metadata::flat::UniversalEntityLike;
use crate::metadata::flat::UniversalFlatEntityMaps;
use crate::metadata::flat::UniversalFlatSearchFieldMetadata;
use crate::metadata::flat::UniversalWorkspaceMaps;

pub struct SearchFieldMetadataActionsBuilder;

pub const SEARCH_FIELD_METADATA_NOT_FOUND: &str = "SEARCH_FIELD_METADATA_NOT_FOUND";
pub const INVALID_SEARCH_FIELD_METADATA_DATA: &str = "INVALID_SEARCH_FIELD_METADATA_DATA";
pub const OBJECT_METADATA_NOT_FOUND: &str = "OBJECT_METADATA_NOT_FOUND";
pub const FIELD_METADATA_NOT_FOUND: &str = "FIELD_METADATA_NOT_FOUND";
pub const TS_VECTOR_FIELD_METADATA_NOT_FOUND: &str = "TS_VECTOR_FIELD_METADATA_NOT_FOUND";
pub const INVALID_TS_VECTOR_FIELD_METADATA: &str = "INVALID_TS_VECTOR_FIELD_METADATA";

impl EntityActionsBuilder for SearchFieldMetadataActionsBuilder {
    type Entity = UniversalFlatSearchFieldMetadata;

    fn metadata_name(&self) -> MetadataName {
        MetadataName::SearchFieldMetadata
    }

    fn maps_of<'m>(
        &self,
        maps: &'m UniversalWorkspaceMaps,
    ) -> &'m UniversalFlatEntityMaps<Self::Entity> {
        &maps.search_field_metadata_maps
    }

    fn maps_of_mut<'m>(
        &self,
        maps: &'m mut UniversalWorkspaceMaps,
    ) -> &'m mut UniversalFlatEntityMaps<Self::Entity> {
        &mut maps.search_field_metadata_maps
    }

    fn validate_creation(
        &self,
        context: &CreationValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let search_field = &context.flat_entity_to_validate;
        let mut errors: Vec<crate::metadata::builders::FlatEntityValidationError> = Vec::new();

        if context
            .optimistic_maps
            .search_field_metadata_maps
            .by_universal_identifier
            .contains_key(&search_field.universal_identifier)
        {
            errors.push(error(
                INVALID_SEARCH_FIELD_METADATA_DATA,
                format!(
                    "Search field metadata with id {} already exists",
                    search_field.universal_identifier
                ),
                Some("Search field metadata already exists"),
                None,
            ));
        }

        let indexed_field = context
            .optimistic_maps
            .field_metadata_maps
            .find_by_universal_identifier(&search_field.field_metadata_universal_identifier);

        if indexed_field.is_none() {
            errors.push(error(
                FIELD_METADATA_NOT_FOUND,
                "Field metadata not found",
                Some("Field metadata not found"),
                None,
            ));
        }

        let ts_vector_field = context
            .optimistic_maps
            .field_metadata_maps
            .find_by_universal_identifier(
                &search_field.ts_vector_field_metadata_universal_identifier,
            );

        match ts_vector_field {
            None => {
                errors.push(error(
                    TS_VECTOR_FIELD_METADATA_NOT_FOUND,
                    "TS_VECTOR field metadata not found",
                    Some("Search vector field not found"),
                    None,
                ));
            }
            Some(ts_vector_field) => {
                if ts_vector_field.field_type != crate::metadata::flat::FieldMetadataType::TsVector
                {
                    errors.push(error(
                        INVALID_TS_VECTOR_FIELD_METADATA,
                        "TS_VECTOR field metadata must be of type TS_VECTOR",
                        Some("Search vector field must be a search vector"),
                        None,
                    ));
                }

                if ts_vector_field.object_metadata_universal_identifier
                    != search_field.object_metadata_universal_identifier
                {
                    errors.push(error(
                        INVALID_TS_VECTOR_FIELD_METADATA,
                        "TS_VECTOR field metadata must belong to the same object",
                        Some("Search vector field must belong to the same object"),
                        None,
                    ));
                }
            }
        }

        let object = context
            .optimistic_maps
            .object_metadata_maps
            .find_by_universal_identifier(&search_field.object_metadata_universal_identifier);

        let Some(object) = object else {
            if errors.is_empty() {
                return Err(FailedFlatEntityValidation {
                    validation_type: "create".to_string(),
                    metadata_name: Some(self.metadata_name()),
                    errors: vec![error(
                        OBJECT_METADATA_NOT_FOUND,
                        "Object metadata not found",
                        Some("Object metadata not found"),
                        None,
                    )],
                    flat_entity_minimal_information: serde_json::json!({
                        "universalIdentifier": search_field.universal_identifier,
                        "objectMetadataUniversalIdentifier":
                            search_field.object_metadata_universal_identifier
                    }),
                });
            }

            return Err(FailedFlatEntityValidation {
                validation_type: "create".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": search_field.universal_identifier,
                    "objectMetadataUniversalIdentifier":
                        search_field.object_metadata_universal_identifier
                }),
            });
        };

        let equivalent_exists = object
            .search_field_metadata_universal_identifiers
            .iter()
            .filter_map(|search_field_universal_identifier| {
                context
                    .optimistic_maps
                    .search_field_metadata_maps
                    .find_by_universal_identifier(search_field_universal_identifier)
            })
            .any(|other| {
                other.universal_identifier != search_field.universal_identifier
                    && other.field_metadata_universal_identifier
                        == search_field.field_metadata_universal_identifier
            });

        if equivalent_exists {
            errors.push(error(
                INVALID_SEARCH_FIELD_METADATA_DATA,
                "Search field metadata with same fieldMetadataId and objectMetadataId already exists",
                Some("Search field metadata already exists"),
                None,
            ));
        }

        if !errors.is_empty() {
            return Err(FailedFlatEntityValidation {
                validation_type: "create".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors,
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": search_field.universal_identifier,
                    "objectMetadataUniversalIdentifier":
                        search_field.object_metadata_universal_identifier
                }),
            });
        }

        Ok(UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::SearchFieldMetadata,
            flat_entity: Some(
                crate::metadata::actions::UniversalFlatEntity::SearchFieldMetadata(Box::new(
                    search_field.clone(),
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
            .search_field_metadata_maps
            .find_by_universal_identifier(universal_identifier)
            .is_none()
        {
            return Err(FailedFlatEntityValidation {
                validation_type: "delete".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    SEARCH_FIELD_METADATA_NOT_FOUND,
                    "Search field metadata to delete not found",
                    Some("Search field metadata to update not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        }

        Ok(UniversalAction::Delete(UniversalDeleteAction {
            metadata_name: MetadataName::SearchFieldMetadata,
            universal_identifier: universal_identifier.to_string(),
            flat_entity: None,
        }))
    }

    fn validate_update(
        &self,
        context: &UpdateValidationContext<Self>,
    ) -> Result<UniversalAction, FailedFlatEntityValidation> {
        let universal_identifier = context.universal_identifier;

        if context
            .optimistic_maps
            .search_field_metadata_maps
            .find_by_universal_identifier(universal_identifier)
            .is_none()
        {
            return Err(FailedFlatEntityValidation {
                validation_type: "update".to_string(),
                metadata_name: Some(self.metadata_name()),
                errors: vec![error(
                    SEARCH_FIELD_METADATA_NOT_FOUND,
                    "Search field metadata to update not found",
                    Some("Search field metadata to update not found"),
                    None,
                )],
                flat_entity_minimal_information: serde_json::json!({
                    "universalIdentifier": universal_identifier
                }),
            });
        }

        Ok(UniversalAction::Update(UniversalUpdateAction {
            metadata_name: MetadataName::SearchFieldMetadata,
            universal_identifier: universal_identifier.to_string(),
            update: context.flat_entity_update.clone(),
            ..UniversalUpdateAction::default()
        }))
    }
}

impl UniversalEntityLike for UniversalFlatSearchFieldMetadata {
    fn universal_identifier(&self) -> &str {
        &self.universal_identifier
    }

    fn merge_update(&self, update: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut payload = serde_json::to_value(self)?;
        let object = payload.as_object_mut().ok_or_else(|| {
            serde::de::Error::custom("search field entity must serialize to a map")
        })?;

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
