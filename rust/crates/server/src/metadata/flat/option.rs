#![allow(dead_code)]

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use super::timestamp;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatFieldMetadataOption {
    #[serde(default)]
    pub id: Uuid,
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_id: Uuid,
    #[serde(default)]
    pub workspace_id: Uuid,
    #[serde(default)]
    pub field_metadata_id: Uuid,
    #[serde(default)]
    pub field_metadata_universal_identifier: String,
    pub value: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub position: f64,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub overrides: Option<serde_json::Value>,
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

impl Default for FlatFieldMetadataOption {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            universal_identifier: String::new(),
            application_id: Uuid::nil(),
            workspace_id: Uuid::nil(),
            field_metadata_id: Uuid::nil(),
            field_metadata_universal_identifier: String::new(),
            value: String::new(),
            label: String::new(),
            color: None,
            description: None,
            position: 0.0,
            is_active: true,
            overrides: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            application_universal_identifier: String::new(),
        }
    }
}

impl FlatFieldMetadataOption {
    pub fn minimal_information() -> FieldMetadataOptionMinimalInformation {
        FieldMetadataOptionMinimalInformation::default()
    }

    pub fn universal(self) -> UniversalFlatFieldMetadataOption {
        UniversalFlatFieldMetadataOption {
            universal_identifier: self.universal_identifier,
            application_universal_identifier: self.application_universal_identifier,
            field_metadata_universal_identifier: self.field_metadata_universal_identifier,
            value: self.value,
            label: self.label,
            color: self.color,
            description: self.description,
            position: self.position,
            is_active: self.is_active,
            overrides: self.overrides,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniversalFlatFieldMetadataOption {
    #[serde(default)]
    pub universal_identifier: String,
    #[serde(default)]
    pub application_universal_identifier: String,
    #[serde(default)]
    pub field_metadata_universal_identifier: String,
    pub value: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub position: f64,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub overrides: Option<serde_json::Value>,
    #[serde(default, with = "timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default, with = "timestamp")]
    pub updated_at: DateTime<Utc>,
}

impl Default for UniversalFlatFieldMetadataOption {
    fn default() -> Self {
        FlatFieldMetadataOption::default().universal()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldMetadataOptionMinimalInformation {
    #[serde(default)]
    pub universal_identifier: String,
}
