#![allow(dead_code)]

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use super::timestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatSearchFieldMetadata {
    #[serde(default)]
    pub id: Uuid,
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_id: Uuid,
    #[serde(default)]
    pub workspace_id: Uuid,
    #[serde(default)]
    pub object_metadata_id: Uuid,
    #[serde(default)]
    pub object_metadata_universal_identifier: String,
    #[serde(default)]
    pub field_metadata_id: Uuid,
    #[serde(default)]
    pub field_metadata_universal_identifier: String,
    #[serde(default)]
    pub ts_vector_field_metadata_id: Uuid,
    #[serde(default)]
    pub ts_vector_field_metadata_universal_identifier: String,
    #[serde(default)]
    pub position: f64,
    #[serde(default = "default_true")]
    pub is_system_side_effect: bool,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub application_universal_identifier: String,
}

fn default_true() -> bool {
    true
}

impl Default for FlatSearchFieldMetadata {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            universal_identifier: String::new(),
            application_id: Uuid::nil(),
            workspace_id: Uuid::nil(),
            object_metadata_id: Uuid::nil(),
            object_metadata_universal_identifier: String::new(),
            field_metadata_id: Uuid::nil(),
            field_metadata_universal_identifier: String::new(),
            ts_vector_field_metadata_id: Uuid::nil(),
            ts_vector_field_metadata_universal_identifier: String::new(),
            position: 0.0,
            is_system_side_effect: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            application_universal_identifier: String::new(),
        }
    }
}

impl FlatSearchFieldMetadata {
    pub fn universal(self) -> UniversalFlatSearchFieldMetadata {
        UniversalFlatSearchFieldMetadata {
            universal_identifier: self.universal_identifier,
            application_universal_identifier: self.application_universal_identifier,
            object_metadata_universal_identifier: self.object_metadata_universal_identifier,
            field_metadata_universal_identifier: self.field_metadata_universal_identifier,
            ts_vector_field_metadata_universal_identifier: self
                .ts_vector_field_metadata_universal_identifier,
            position: self.position,
            is_system_side_effect: self.is_system_side_effect,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniversalFlatSearchFieldMetadata {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_universal_identifier: String,
    #[serde(default)]
    pub object_metadata_universal_identifier: String,
    #[serde(default)]
    pub field_metadata_universal_identifier: String,
    #[serde(default)]
    pub ts_vector_field_metadata_universal_identifier: String,
    #[serde(default)]
    pub position: f64,
    #[serde(default = "default_true")]
    pub is_system_side_effect: bool,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
}

impl Default for UniversalFlatSearchFieldMetadata {
    fn default() -> Self {
        FlatSearchFieldMetadata::default().universal()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchFieldMetadataMinimalInformation {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_metadata_universal_identifier: Option<String>,
}
