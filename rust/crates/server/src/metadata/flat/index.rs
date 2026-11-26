#![allow(dead_code)]

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use super::enums::IndexType;
use super::timestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatIndexFieldMetadata {
    #[serde(default)]
    pub id: Uuid,
    #[serde(default)]
    pub workspace_id: Uuid,
    #[serde(default)]
    pub index_metadata_id: Uuid,
    #[serde(default)]
    pub field_metadata_id: Uuid,
    #[serde(default)]
    pub order: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_field_name: Option<String>,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UniversalFlatIndexFieldMetadata {
    #[serde(default)]
    pub order: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_field_name: Option<String>,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub index_metadata_universal_identifier: String,
    #[serde(default)]
    pub field_metadata_universal_identifier: String,
}

impl FlatIndexFieldMetadata {
    pub fn universal(
        self,
        index_metadata_universal_identifier: String,
    ) -> UniversalFlatIndexFieldMetadata {
        UniversalFlatIndexFieldMetadata {
            order: self.order,
            sub_field_name: self.sub_field_name,
            created_at: self.created_at,
            updated_at: self.updated_at,
            index_metadata_universal_identifier,
            field_metadata_universal_identifier: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FlatIndexMetadata {
    #[serde(default)]
    pub id: Uuid,
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_id: Uuid,
    #[serde(default)]
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub object_metadata_id: Uuid,
    #[serde(default)]
    pub object_metadata_universal_identifier: String,
    #[serde(default)]
    pub is_unique: bool,
    #[serde(default)]
    pub index_where_clause: Option<String>,
    #[serde(default)]
    pub index_type: IndexType,
    #[serde(default)]
    pub is_system_side_effect: bool,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub flat_index_field_metadatas: Vec<FlatIndexFieldMetadata>,
    #[serde(default)]
    pub universal_flat_index_field_metadatas: Vec<UniversalFlatIndexFieldMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UniversalFlatIndexMetadata {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_universal_identifier: String,
    pub name: String,
    #[serde(default)]
    pub object_metadata_universal_identifier: String,
    #[serde(default)]
    pub is_unique: bool,
    #[serde(default)]
    pub index_where_clause: Option<String>,
    #[serde(default)]
    pub index_type: IndexType,
    #[serde(default)]
    pub is_system_side_effect: bool,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub universal_flat_index_field_metadatas: Vec<UniversalFlatIndexFieldMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IndexMinimalInformation {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl UniversalFlatIndexMetadata {
    pub fn minimal_information(&self) -> IndexMinimalInformation {
        IndexMinimalInformation {
            universal_identifier: self.universal_identifier.clone(),
            name: Some(self.name.clone()),
        }
    }
}
