#![allow(dead_code)]

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use super::field::FlatFieldMetadata;
use super::field::UniversalFlatFieldMetadata;
use super::index::FlatIndexMetadata;
use super::index::UniversalFlatIndexMetadata;
use super::object::FlatObjectMetadata;
use super::object::UniversalFlatObjectMetadata;
use super::option::FlatFieldMetadataOption;
use super::option::UniversalFlatFieldMetadataOption;
use super::search_field::FlatSearchFieldMetadata;
use super::search_field::UniversalFlatSearchFieldMetadata;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniversalFlatEntityMaps<T> {
    #[serde(default)]
    pub by_universal_identifier: HashMap<String, T>,
}

impl<T> Default for UniversalFlatEntityMaps<T> {
    fn default() -> Self {
        UniversalFlatEntityMaps {
            by_universal_identifier: HashMap::new(),
        }
    }
}

impl<T: Clone> UniversalFlatEntityMaps<T> {
    pub fn find_by_universal_identifier(&self, universal_identifier: &str) -> Option<&T> {
        self.by_universal_identifier.get(universal_identifier)
    }

    pub fn add(&mut self, universal_identifier: impl Into<String>, entity: T) {
        self.by_universal_identifier
            .insert(universal_identifier.into(), entity);
    }

    pub fn delete_by_universal_identifier(&mut self, universal_identifier: &str) -> Option<T> {
        self.by_universal_identifier.remove(universal_identifier)
    }

    pub fn entities(&self) -> Vec<&T> {
        self.by_universal_identifier.values().collect()
    }

    pub fn universal_identifiers(&self) -> Vec<&String> {
        self.by_universal_identifier.keys().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlatEntityMaps<T> {
    #[serde(default)]
    pub by_universal_identifier: HashMap<String, T>,
    #[serde(default)]
    pub universal_identifier_by_id: HashMap<String, String>,
    #[serde(default)]
    pub universal_identifiers_by_application_id: HashMap<String, Vec<String>>,
}

impl<T> Default for FlatEntityMaps<T> {
    fn default() -> Self {
        FlatEntityMaps {
            by_universal_identifier: HashMap::new(),
            universal_identifier_by_id: HashMap::new(),
            universal_identifiers_by_application_id: HashMap::new(),
        }
    }
}

impl<T: Clone> FlatEntityMaps<T> {
    pub fn find_by_universal_identifier(&self, universal_identifier: &str) -> Option<&T> {
        self.by_universal_identifier.get(universal_identifier)
    }

    pub fn find_by_id(&self, id: &str) -> Option<&T> {
        self.universal_identifier_by_id
            .get(id)
            .and_then(|universal_identifier| {
                self.find_by_universal_identifier(universal_identifier)
            })
    }

    pub fn universal_identifier_by_id(&self, id: &str) -> Option<&str> {
        self.universal_identifier_by_id.get(id).map(String::as_str)
    }

    pub fn add(
        &mut self,
        id: impl Into<String>,
        entity: T,
        universal_identifier: impl Into<String>,
        application_id: impl Into<String>,
    ) {
        let universal_identifier = universal_identifier.into();
        let application_id = application_id.into();

        self.by_universal_identifier
            .insert(universal_identifier.clone(), entity);
        self.universal_identifier_by_id
            .insert(id.into(), universal_identifier.clone());
        self.universal_identifiers_by_application_id
            .entry(application_id)
            .or_default()
            .push(universal_identifier);
    }

    pub fn delete_by_universal_identifier(&mut self, universal_identifier: &str) -> Option<T> {
        let removed = self.by_universal_identifier.remove(universal_identifier);

        if removed.is_some() {
            self.universal_identifier_by_id
                .retain(|_, value| value != universal_identifier);
        }

        removed
    }
}

#[derive(Debug, Clone, Default)]
pub struct FlatWorkspaceMaps {
    pub object_metadata_maps: FlatEntityMaps<FlatObjectMetadata>,
    pub field_metadata_maps: FlatEntityMaps<FlatFieldMetadata>,
    pub index_metadata_maps: FlatEntityMaps<FlatIndexMetadata>,
    pub field_metadata_option_maps: FlatEntityMaps<FlatFieldMetadataOption>,
    pub search_field_metadata_maps: FlatEntityMaps<FlatSearchFieldMetadata>,
}

#[derive(Debug, Clone, Default)]
pub struct UniversalWorkspaceMaps {
    pub object_metadata_maps: UniversalFlatEntityMaps<UniversalFlatObjectMetadata>,
    pub field_metadata_maps: UniversalFlatEntityMaps<UniversalFlatFieldMetadata>,
    pub index_metadata_maps: UniversalFlatEntityMaps<UniversalFlatIndexMetadata>,
    pub field_metadata_option_maps: UniversalFlatEntityMaps<UniversalFlatFieldMetadataOption>,
    pub search_field_metadata_maps: UniversalFlatEntityMaps<UniversalFlatSearchFieldMetadata>,
}

#[derive(Debug, Clone, Default)]
pub struct FromToMaps<T> {
    pub from: T,
    pub to: T,
}

#[derive(Debug, Clone, Default)]
pub struct FromToAllMaps {
    pub object_metadata: FromToMaps<UniversalFlatEntityMaps<UniversalFlatObjectMetadata>>,
    pub field_metadata: FromToMaps<UniversalFlatEntityMaps<UniversalFlatFieldMetadata>>,
    pub index_metadata: FromToMaps<UniversalFlatEntityMaps<UniversalFlatIndexMetadata>>,
    pub field_metadata_option:
        FromToMaps<UniversalFlatEntityMaps<UniversalFlatFieldMetadataOption>>,
    pub search_field_metadata:
        FromToMaps<UniversalFlatEntityMaps<UniversalFlatSearchFieldMetadata>>,
}
