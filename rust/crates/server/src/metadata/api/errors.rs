use serde::Serialize;
use serde_json::json;
use serde_json::Value;

use crate::metadata::builders::FailedFlatEntityValidation;
use crate::metadata::builders::OrchestratorFailureReport;
use crate::workspace_migrations::runner::RunnerError;

pub const GRAPHQL_VALIDATION_FAILED: &str = "GRAPHQL_VALIDATION_FAILED";
pub const GRAPHQL_PARSE_FAILED: &str = "GRAPHQL_PARSE_FAILED";
pub const BAD_USER_INPUT: &str = "BAD_USER_INPUT";
pub const NOT_FOUND: &str = "NOT_FOUND";
pub const FORBIDDEN: &str = "FORBIDDEN";
pub const CONFLICT: &str = "CONFLICT";
pub const INTERNAL_SERVER_ERROR: &str = "INTERNAL_SERVER_ERROR";
pub const UNAUTHENTICATED: &str = "UNAUTHENTICATED";
pub const METHOD_NOT_ALLOWED: &str = "METHOD_NOT_ALLOWED";
pub const METADATA_VALIDATION_FAILED: &str = "METADATA_VALIDATION_FAILED";
pub const APPLICATION_INSTALLATION_FAILED: &str = "APPLICATION_INSTALLATION_FAILED";

#[derive(Debug, Clone)]
pub struct GraphQlError {
    pub message: String,
    pub code: &'static str,
    pub extensions: Value,
}

impl GraphQlError {
    pub fn new(message: impl Into<String>, code: &'static str) -> Self {
        Self {
            message: message.into(),
            code,
            extensions: json!({}),
        }
    }

    pub fn with_extensions(mut self, extensions: Value) -> Self {
        self.extensions = extensions;
        self
    }

    pub fn with_sub_code(mut self, sub_code: &str) -> Self {
        if !self.extensions.is_object() {
            self.extensions = json!({});
        }

        if let Some(object) = self.extensions.as_object_mut() {
            object.insert("subCode".to_string(), json!(sub_code));
        }

        self
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(message, GRAPHQL_VALIDATION_FAILED)
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(message, GRAPHQL_PARSE_FAILED)
    }

    pub fn user_input(message: impl Into<String>) -> Self {
        Self::new(message, BAD_USER_INPUT)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(message, NOT_FOUND)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(message, FORBIDDEN)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(message, CONFLICT)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(message, INTERNAL_SERVER_ERROR)
    }

    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(message, UNAUTHENTICATED)
    }

    pub fn method_not_allowed(message: impl Into<String>) -> Self {
        Self::new(message, METHOD_NOT_ALLOWED)
    }

    pub fn builder_failure(message: impl Into<String>, report: &OrchestratorFailureReport) -> Self {
        let mut extensions = metadata_validation_error_payload(report);

        if let Some(object) = extensions.as_object_mut() {
            object.insert("code".to_string(), json!("METADATA_VALIDATION_ERROR"));
        }

        Self {
            message: message.into(),
            code: METADATA_VALIDATION_FAILED,
            extensions,
        }
    }

    pub fn runner(error: &RunnerError) -> Self {
        let message = error.message.clone();

        match error.code {
            "DEFERRED_WORKSPACE_MIGRATION_ACTIONS_IN_PROGRESS" => {
                Self::conflict(message).with_sub_code(error.code)
            }
            "EXECUTION_FAILED" => Self::new(message, APPLICATION_INSTALLATION_FAILED),
            _ => Self::new(message, INTERNAL_SERVER_ERROR),
        }
    }

    pub fn to_json(&self) -> Value {
        let mut extensions = match self.extensions.clone() {
            Value::Object(object) => object,
            _ => serde_json::Map::new(),
        };

        let code = self.code.to_string();

        extensions.insert("code".to_string(), json!(code));

        json!({
            "message": self.message,
            "extensions": extensions,
        })
    }
}

impl Serialize for GraphQlError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut extensions = match self.extensions.clone() {
            Value::Object(object) => object,
            _ => serde_json::Map::new(),
        };

        let code = self.code.to_string();

        extensions.insert("code".to_string(), json!(code));

        json!({
            "message": self.message,
            "extensions": extensions,
        })
        .serialize(serializer)
    }
}

pub fn metadata_validation_error_payload(report: &OrchestratorFailureReport) -> Value {
    let mut errors = serde_json::Map::new();
    let mut summary = serde_json::Map::new();
    let mut total_errors: usize = 0;

    for metadata_name in crate::metadata::actions::MetadataName::all() {
        let Some(failures) = report.failures_for(*metadata_name) else {
            continue;
        };

        if failures.is_empty() {
            continue;
        }

        let count = failures
            .iter()
            .map(|failure| failure.errors.len())
            .sum::<usize>();

        total_errors += count;

        let serialized = serde_json::to_value(failures).unwrap_or(Value::Null);

        errors.insert(metadata_name.as_str().to_string(), serialized);
        summary.insert(metadata_name.as_str().to_string(), json!(count));
    }

    summary.insert("totalErrors".to_string(), json!(total_errors));

    json!({
        "errors": errors,
        "summary": summary,
        "message": validation_summary_message(&summary),
    })
}

fn validation_summary_message(summary: &serde_json::Map<String, Value>) -> String {
    let mut parts: Vec<String> = Vec::new();

    for metadata_name in crate::metadata::actions::MetadataName::all() {
        let Some(count) = summary.get(metadata_name.as_str()).and_then(Value::as_u64) else {
            continue;
        };

        if count == 0 {
            continue;
        }

        let label = metadata_name.as_str();
        let pluralized = if count == 1 {
            label.to_string()
        } else {
            pluralize_label(label)
        };

        parts.push(format!("{count} {pluralized}"));
    }

    if parts.is_empty() {
        return "Metadata validation failed".to_string();
    }

    format!("Validation failed for {}", parts.join(", "))
}

fn pluralize_label(label: &str) -> String {
    if label.ends_with('y') && !label.ends_with("ay") && !label.ends_with("ey") {
        let mut stem = label.to_string();

        stem.pop();

        return format!("{stem}ies");
    }

    if label.ends_with('s')
        || label.ends_with('x')
        || label.ends_with("ch")
        || label.ends_with("sh")
    {
        return format!("{label}es");
    }

    format!("{label}s")
}

pub fn first_failure_message(failures: &[FailedFlatEntityValidation]) -> Option<String> {
    failures
        .iter()
        .find_map(|failure| failure.errors.first())
        .map(|error| error.message.clone())
}

pub fn field_transpilation_failure(
    domain_code: &str,
    message: &str,
    validation_type: &str,
    outer_message: &str,
) -> GraphQlError {
    let mut report = OrchestratorFailureReport::default();

    report.push_all(
        crate::metadata::actions::MetadataName::FieldMetadata,
        vec![FailedFlatEntityValidation {
            validation_type: validation_type.to_string(),
            metadata_name: Some(crate::metadata::actions::MetadataName::FieldMetadata),
            errors: vec![crate::metadata::builders::FlatEntityValidationError {
                code: domain_code.to_string(),
                message: message.to_string(),
                user_friendly_message: Some(message.to_string()),
                value: None,
            }],
            flat_entity_minimal_information: serde_json::json!({ "id": "" }),
        }],
    );

    GraphQlError::builder_failure(outer_message, &report)
}

pub fn object_transpilation_failure(
    domain_code: &str,
    message: &str,
    validation_type: &str,
    outer_message: &str,
) -> GraphQlError {
    let mut report = OrchestratorFailureReport::default();

    report.push_all(
        crate::metadata::actions::MetadataName::ObjectMetadata,
        vec![FailedFlatEntityValidation {
            validation_type: validation_type.to_string(),
            metadata_name: Some(crate::metadata::actions::MetadataName::ObjectMetadata),
            errors: vec![crate::metadata::builders::FlatEntityValidationError {
                code: domain_code.to_string(),
                message: message.to_string(),
                user_friendly_message: Some(message.to_string()),
                value: None,
            }],
            flat_entity_minimal_information: serde_json::json!({ "id": "" }),
        }],
    );

    GraphQlError::builder_failure(outer_message, &report)
}

pub fn field_domain_graphql_error(domain_code: &str, message: &str) -> GraphQlError {
    let error = match domain_code {
        "FIELD_METADATA_NOT_FOUND" => GraphQlError::not_found(message),
        "INVALID_FIELD_INPUT" => GraphQlError::user_input(message),
        "FIELD_MUTATION_NOT_ALLOWED" => GraphQlError::forbidden(message),
        "FIELD_ALREADY_EXISTS" => GraphQlError::conflict(message),
        _ => GraphQlError::internal(message),
    };

    error.with_sub_code(domain_code)
}

pub fn object_domain_graphql_error(domain_code: &str, message: &str) -> GraphQlError {
    let error = match domain_code {
        "OBJECT_METADATA_NOT_FOUND" => GraphQlError::not_found(message),
        "INVALID_OBJECT_INPUT" => GraphQlError::user_input(message),
        "OBJECT_MUTATION_NOT_ALLOWED" | "NAME_CONFLICT" => GraphQlError::forbidden(message),
        "OBJECT_ALREADY_EXISTS" => GraphQlError::conflict(message),
        _ => GraphQlError::internal(message),
    };

    error.with_sub_code(domain_code)
}

pub fn index_domain_graphql_error(domain_code: &str, message: &str) -> GraphQlError {
    let error = match domain_code {
        "INDEX_NOT_FOUND" | "INDEX_OBJECT_NOT_FOUND" => GraphQlError::not_found(message),
        "INDEX_FIELDS_REQUIRED"
        | "DUPLICATE_INDEX_FIELDS"
        | "INDEX_FIELD_NOT_FOUND_ON_OBJECT"
        | "INDEX_NOT_SUPPORTED_FOR_COMPOSITE_FIELD"
        | "INDEX_NOT_SUPPORTED_FOR_MORH_RELATION_FIELD_AND_RELATION_FIELD"
        | "INDEX_TYPE_NOT_SUPPORTED_FOR_FIELD_TYPE"
        | "DUPLICATE_UNIQUE_INDEX" => GraphQlError::user_input(message),
        "CANNOT_DELETE_SYSTEM_INDEX" => GraphQlError::forbidden(message),
        "CUSTOM_INDEX_LIMIT_REACHED" => GraphQlError::conflict(message),
        _ => GraphQlError::internal(message),
    };

    error.with_sub_code(domain_code)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::metadata::actions::MetadataName;
    use crate::metadata::builders::FailedFlatEntityValidation;
    use crate::metadata::builders::FlatEntityValidationError;

    fn failure(code: &str, message: &str) -> FailedFlatEntityValidation {
        FailedFlatEntityValidation {
            validation_type: "create".to_string(),
            metadata_name: Some(MetadataName::ObjectMetadata),
            errors: vec![FlatEntityValidationError {
                code: code.to_string(),
                message: message.to_string(),
                user_friendly_message: Some(message.to_string()),
                value: None,
            }],
            flat_entity_minimal_information: json!({"universalIdentifier": "u"}),
        }
    }

    #[test]
    fn serialization_carries_the_category_code_in_the_extensions() {
        let error = GraphQlError::user_input("bad name");
        let value = serde_json::to_value(&error).unwrap();

        assert_eq!(value["message"], json!("bad name"));
        assert_eq!(value["extensions"]["code"], json!("BAD_USER_INPUT"));
    }

    #[test]
    fn the_extensions_code_is_the_category_like_the_upstream_constructor() {
        let error = GraphQlError::new("locked", INTERNAL_SERVER_ERROR)
            .with_extensions(json!({ "code": "DDL_LOCKED" }));
        let value = serde_json::to_value(&error).unwrap();

        assert_eq!(value["extensions"]["code"], json!("INTERNAL_SERVER_ERROR"));
    }

    #[test]
    fn runner_codes_map_like_the_upstream_formatter() {
        let deferred = RunnerError::deferred_in_progress();
        let mapped = GraphQlError::runner(&deferred);
        let value = serde_json::to_value(&mapped).unwrap();

        assert_eq!(value["extensions"]["code"], json!("CONFLICT"));
        assert_eq!(
            value["extensions"]["subCode"],
            json!("DEFERRED_WORKSPACE_MIGRATION_ACTIONS_IN_PROGRESS")
        );

        let execution = RunnerError::execution_failed("boom");
        let mapped = GraphQlError::runner(&execution);
        let value = serde_json::to_value(&mapped).unwrap();

        assert_eq!(
            value["extensions"]["code"],
            json!("APPLICATION_INSTALLATION_FAILED")
        );

        let locked = RunnerError::ddl_locked();
        let mapped = GraphQlError::runner(&locked);
        let value = serde_json::to_value(&mapped).unwrap();

        assert_eq!(value["extensions"]["code"], json!("INTERNAL_SERVER_ERROR"));
    }

    #[test]
    fn builder_failures_become_metadata_validation_errors() {
        let mut report = OrchestratorFailureReport::default();

        report.push_all(
            MetadataName::ObjectMetadata,
            vec![failure("INVALID_OBJECT_INPUT", "Name is too long")],
        );

        let error = GraphQlError::builder_failure("Multiple validation errors occurred", &report);
        let value = serde_json::to_value(&error).unwrap();

        assert_eq!(
            value["extensions"]["code"],
            json!("METADATA_VALIDATION_FAILED")
        );
        assert_eq!(value["extensions"]["summary"]["totalErrors"], json!(1));
        assert_eq!(value["extensions"]["summary"]["objectMetadata"], json!(1));
        assert_eq!(
            value["extensions"]["errors"]["objectMetadata"][0]["errors"][0]["code"],
            json!("INVALID_OBJECT_INPUT")
        );
        assert_eq!(
            value["extensions"]["message"],
            json!("Validation failed for 1 objectMetadata")
        );
        assert_eq!(error.message, "Multiple validation errors occurred");
    }

    #[test]
    fn domain_code_mappings_match_the_handlers() {
        let value =
            serde_json::to_value(field_domain_graphql_error("FIELD_METADATA_NOT_FOUND", "x"))
                .unwrap();
        assert_eq!(value["extensions"]["code"], json!("NOT_FOUND"));
        assert_eq!(
            value["extensions"]["subCode"],
            json!("FIELD_METADATA_NOT_FOUND")
        );

        assert_eq!(
            field_domain_graphql_error("INVALID_FIELD_INPUT", "x").code,
            "BAD_USER_INPUT"
        );
        assert_eq!(
            field_domain_graphql_error("FIELD_MUTATION_NOT_ALLOWED", "x").code,
            "FORBIDDEN"
        );
        assert_eq!(
            object_domain_graphql_error("OBJECT_ALREADY_EXISTS", "x").code,
            "CONFLICT"
        );
        assert_eq!(
            object_domain_graphql_error("NAME_CONFLICT", "x").code,
            "FORBIDDEN"
        );
        assert_eq!(
            index_domain_graphql_error("CANNOT_DELETE_SYSTEM_INDEX", "x").code,
            "FORBIDDEN"
        );
        assert_eq!(
            index_domain_graphql_error("CUSTOM_INDEX_LIMIT_REACHED", "x").code,
            "CONFLICT"
        );
        assert_eq!(
            index_domain_graphql_error("INDEX_FIELDS_REQUIRED", "x").code,
            "BAD_USER_INPUT"
        );
    }

    #[test]
    fn pluralizes_labels_like_the_upstream_helper() {
        assert_eq!(pluralize_label("objectMetadata"), "objectMetadatas");
        assert_eq!(pluralize_label("index"), "indexes");
        assert_eq!(pluralize_label("activity"), "activities");
    }
}
