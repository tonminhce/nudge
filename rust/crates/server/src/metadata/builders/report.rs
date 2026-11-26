#![allow(dead_code)]

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use crate::metadata::actions::MetadataName;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatEntityValidationError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_friendly_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedFlatEntityValidation {
    #[serde(rename = "type")]
    pub validation_type: String,
    #[serde(default)]
    pub metadata_name: Option<MetadataName>,
    #[serde(default)]
    pub errors: Vec<FlatEntityValidationError>,
    #[serde(default)]
    pub flat_entity_minimal_information: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorFailureReport {
    #[serde(default, flatten)]
    pub by_metadata_name: HashMap<MetadataName, Vec<FailedFlatEntityValidation>>,
}

impl OrchestratorFailureReport {
    pub fn push_all(
        &mut self,
        metadata_name: MetadataName,
        failures: Vec<FailedFlatEntityValidation>,
    ) {
        if failures.is_empty() {
            return;
        }

        self.by_metadata_name
            .entry(metadata_name)
            .or_default()
            .extend(failures);
    }

    pub fn merge(&mut self, other: OrchestratorFailureReport) {
        for (metadata_name, failures) in other.by_metadata_name {
            self.push_all(metadata_name, failures);
        }
    }

    pub fn total_errors(&self) -> usize {
        self.by_metadata_name
            .values()
            .map(|failures| {
                failures
                    .iter()
                    .map(|failure| failure.errors.len())
                    .sum::<usize>()
            })
            .sum()
    }

    pub fn failures_for(
        &self,
        metadata_name: MetadataName,
    ) -> Option<&Vec<FailedFlatEntityValidation>> {
        self.by_metadata_name.get(&metadata_name)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetadataValidationErrorResponse {
    pub summary: MetadataValidationSummary,
    pub errors: OrchestratorFailureReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetadataValidationSummary {
    pub total_errors: usize,
}

impl MetadataValidationErrorResponse {
    pub fn from_report(report: &OrchestratorFailureReport) -> Self {
        MetadataValidationErrorResponse {
            summary: MetadataValidationSummary {
                total_errors: report.total_errors(),
            },
            errors: report.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::FailedFlatEntityValidation;
    use super::FlatEntityValidationError;
    use super::MetadataValidationErrorResponse;
    use super::OrchestratorFailureReport;
    use crate::metadata::actions::MetadataName;

    fn failure(metadata_name: MetadataName) -> FailedFlatEntityValidation {
        FailedFlatEntityValidation {
            validation_type: "create".to_string(),
            metadata_name: Some(metadata_name),
            errors: vec![FlatEntityValidationError {
                code: "INVALID_OBJECT_INPUT".to_string(),
                message: "Object already exists".to_string(),
                user_friendly_message: Some("Object already exists".to_string()),
                value: Some(json!("rocket")),
            }],
            flat_entity_minimal_information: json!({
                "universalIdentifier": "20202020-e674-48e5-a542-72570eee7213",
                "nameSingular": "rocket",
                "namePlural": "rockets"
            }),
        }
    }

    #[test]
    fn failed_metadata_validation_matches_upstream_shape() {
        let mut report = OrchestratorFailureReport::default();

        report.push_all(
            MetadataName::ObjectMetadata,
            vec![failure(MetadataName::ObjectMetadata)],
        );

        let payload = MetadataValidationErrorResponse::from_report(&report);
        let value = serde_json::to_value(&payload).unwrap();

        assert_eq!(
            value
                .get("summary")
                .and_then(|summary| summary.get("totalErrors"))
                .and_then(|total| total.as_u64()),
            Some(1)
        );

        let object_errors = value
            .get("errors")
            .and_then(|errors| errors.get("objectMetadata"))
            .and_then(|failures| failures.as_array())
            .unwrap();

        let first_failure = &object_errors[0];

        assert_eq!(
            first_failure.get("type").and_then(|raw| raw.as_str()),
            Some("create")
        );
        assert_eq!(
            first_failure
                .get("errors")
                .and_then(|errors| errors.as_array())
                .map(|errors| errors.len()),
            Some(1)
        );

        let first_error = &first_failure["errors"][0];

        assert_eq!(
            first_error.get("code").and_then(|raw| raw.as_str()),
            Some("INVALID_OBJECT_INPUT")
        );
        assert_eq!(
            first_error.get("message").and_then(|raw| raw.as_str()),
            Some("Object already exists")
        );
        assert_eq!(
            first_error
                .get("userFriendlyMessage")
                .and_then(|raw| raw.as_str()),
            Some("Object already exists")
        );
    }

    #[test]
    fn total_errors_counts_every_error() {
        let mut report = OrchestratorFailureReport::default();

        let mut double_failure = failure(MetadataName::FieldMetadata);

        double_failure.errors.push(FlatEntityValidationError {
            code: "INVALID_FIELD_INPUT".to_string(),
            message: "second".to_string(),
            user_friendly_message: None,
            value: None,
        });

        report.push_all(MetadataName::FieldMetadata, vec![double_failure]);
        report.push_all(
            MetadataName::ObjectMetadata,
            vec![failure(MetadataName::ObjectMetadata)],
        );

        assert_eq!(report.total_errors(), 3);
    }
}
