#![allow(dead_code)]

use std::collections::HashMap;

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalFlatEntity;
use crate::metadata::flat::UniversalFlatIndexMetadata;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlatEntityToCreateDeleteUpdate {
    pub to_create: Vec<UniversalFlatEntity>,
    pub to_update: Vec<UniversalFlatEntity>,
    pub to_delete: Vec<UniversalFlatEntity>,
}

impl FlatEntityToCreateDeleteUpdate {
    pub fn is_empty(&self) -> bool {
        self.to_create.is_empty() && self.to_update.is_empty() && self.to_delete.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlatEntityOperationsMatrix {
    pub by_metadata_name: HashMap<MetadataName, FlatEntityToCreateDeleteUpdate>,
}

impl FlatEntityOperationsMatrix {
    pub fn bucket_mut(
        &mut self,
        metadata_name: MetadataName,
    ) -> &mut FlatEntityToCreateDeleteUpdate {
        self.by_metadata_name.entry(metadata_name).or_default()
    }

    pub fn bucket(&self, metadata_name: MetadataName) -> Option<&FlatEntityToCreateDeleteUpdate> {
        self.by_metadata_name.get(&metadata_name)
    }

    pub fn is_empty(&self) -> bool {
        self.by_metadata_name
            .values()
            .all(FlatEntityToCreateDeleteUpdate::is_empty)
    }
}

pub struct SideEffectExpansion {
    pub matrix: FlatEntityOperationsMatrix,
    pub cascade_deletion_parent_by_universal_identifier: HashMap<String, String>,
}

pub fn expand_matrix_with_side_effects(
    matrix: FlatEntityOperationsMatrix,
    maps: &crate::metadata::flat::UniversalWorkspaceMaps,
) -> SideEffectExpansion {
    let mut expanded = matrix;
    let mut cascade_deletion_parent_by_universal_identifier: HashMap<String, String> =
        HashMap::new();

    let deleted_fields: Vec<crate::metadata::flat::UniversalFlatFieldMetadata> = expanded
        .bucket(MetadataName::FieldMetadata)
        .map(|bucket| {
            bucket
                .to_delete
                .iter()
                .filter_map(|entity| match entity {
                    UniversalFlatEntity::FieldMetadata(field) => Some((**field).clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();

    if deleted_fields.is_empty() {
        return SideEffectExpansion {
            matrix: expanded,
            cascade_deletion_parent_by_universal_identifier,
        };
    }

    let mut option_deletes: Vec<UniversalFlatEntity> = Vec::new();
    let mut search_field_deletes: Vec<UniversalFlatEntity> = Vec::new();
    let mut index_deletes: Vec<UniversalFlatEntity> = Vec::new();

    for field in &deleted_fields {
        for (universal_identifier, option) in maps
            .field_metadata_option_maps
            .by_universal_identifier
            .iter()
        {
            if option.field_metadata_universal_identifier == field.universal_identifier {
                option_deletes.push(UniversalFlatEntity::FieldMetadataOption(Box::new(
                    option.clone(),
                )));
                cascade_deletion_parent_by_universal_identifier.insert(
                    universal_identifier.clone(),
                    field.universal_identifier.clone(),
                );
            }
        }

        for (universal_identifier, search_field) in maps
            .search_field_metadata_maps
            .by_universal_identifier
            .iter()
        {
            let indexes_the_field = search_field.field_metadata_universal_identifier
                == field.universal_identifier
                || search_field.ts_vector_field_metadata_universal_identifier
                    == field.universal_identifier;

            if indexes_the_field {
                search_field_deletes.push(UniversalFlatEntity::SearchFieldMetadata(Box::new(
                    search_field.clone(),
                )));
                cascade_deletion_parent_by_universal_identifier.insert(
                    universal_identifier.clone(),
                    field.universal_identifier.clone(),
                );
            }
        }

        if field.is_unique && !field.is_relation_or_morph() {
            for (universal_identifier, index) in
                maps.index_metadata_maps.by_universal_identifier.iter()
            {
                if !is_system_unique_index(index) {
                    continue;
                }

                if backs_exactly_one_field(index, &field.universal_identifier) {
                    index_deletes.push(UniversalFlatEntity::IndexMetadata(Box::new(index.clone())));
                    cascade_deletion_parent_by_universal_identifier.insert(
                        universal_identifier.clone(),
                        field.universal_identifier.clone(),
                    );
                }
            }
        }
    }

    if !option_deletes.is_empty() {
        let bucket = expanded.bucket_mut(MetadataName::FieldMetadataOption);

        for option in option_deletes {
            if !bucket
                .to_delete
                .iter()
                .any(|existing| existing.universal_identifier() == option.universal_identifier())
            {
                bucket.to_delete.push(option);
            }
        }
    }

    if !search_field_deletes.is_empty() {
        let bucket = expanded.bucket_mut(MetadataName::SearchFieldMetadata);

        for search_field in search_field_deletes {
            if !bucket.to_delete.iter().any(|existing| {
                existing.universal_identifier() == search_field.universal_identifier()
            }) {
                bucket.to_delete.push(search_field);
            }
        }
    }

    if !index_deletes.is_empty() {
        let bucket = expanded.bucket_mut(MetadataName::Index);

        for index in index_deletes {
            if !bucket
                .to_delete
                .iter()
                .any(|existing| existing.universal_identifier() == index.universal_identifier())
            {
                bucket.to_delete.push(index);
            }
        }
    }

    SideEffectExpansion {
        matrix: expanded,
        cascade_deletion_parent_by_universal_identifier,
    }
}

fn is_system_unique_index(index: &UniversalFlatIndexMetadata) -> bool {
    index.is_system_side_effect && index.is_unique
}

fn backs_exactly_one_field(
    index: &UniversalFlatIndexMetadata,
    field_universal_identifier: &str,
) -> bool {
    let fields = &index.universal_flat_index_field_metadatas;

    fields.len() == 1
        && fields[0].sub_field_name.is_none()
        && fields[0].field_metadata_universal_identifier == field_universal_identifier
}
