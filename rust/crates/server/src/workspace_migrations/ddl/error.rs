#![allow(dead_code)]

use crate::workspace_migrations::column_defs::ColumnDefsError;

#[derive(Debug)]
pub struct DdlError {
    pub code: &'static str,
    pub message: String,
    source: Option<sqlx::Error>,
}

impl DdlError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }

    pub fn enum_operation_failed(message: impl Into<String>) -> Self {
        Self::new("ENUM_OPERATION_FAILED", message)
    }

    pub fn concurrent_index_creation_in_transaction() -> Self {
        Self::new(
            "CONCURRENT_INDEX_CREATION_IN_TRANSACTION",
            "CREATE INDEX CONCURRENTLY cannot run inside a transaction block. Pass a connection with no active transaction.",
        )
    }

    pub fn unsupported_index_type(index_type: &str) -> Self {
        Self::new(
            "UNSUPPORTED_INDEX_TYPE",
            format!("Unsupported index type: {index_type}"),
        )
    }

    pub fn unsupported_index_where_clause(clause: &str) -> Self {
        Self::new(
            "UNSUPPORTED_INDEX_WHERE_CLAUSE",
            format!(
                "Unsupported index WHERE clause: \"{clause}\". Only allowlisted patterns are permitted to prevent SQL injection. Add the pattern to ALLOWED_INDEX_WHERE_CLAUSES after security review."
            ),
        )
    }

    pub fn unsafe_ts_vector_expression() -> Self {
        Self::new(
            "UNSAFE_TS_VECTOR_EXPRESSION",
            "Unsafe tsvector expression detected",
        )
    }

    pub fn unsupported_on_delete_action(action: &str) -> Self {
        Self::new(
            "UNSUPPORTED_ON_DELETE_ACTION",
            format!("Unsupported ON DELETE action: {action}"),
        )
    }

    pub fn unsupported_on_update_action(action: &str) -> Self {
        Self::new(
            "UNSUPPORTED_ON_UPDATE_ACTION",
            format!("Unsupported ON UPDATE action: {action}"),
        )
    }

    pub fn null_byte_in_identifier() -> Self {
        Self::new(
            "INTERNAL_SERVER_ERROR",
            "Null bytes are not allowed in PostgreSQL identifiers",
        )
    }

    pub fn null_byte_in_literal() -> Self {
        Self::new(
            "INTERNAL_SERVER_ERROR",
            "Null bytes are not allowed in PostgreSQL string literals",
        )
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl std::fmt::Display for DdlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DdlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|error| error as &(dyn std::error::Error + 'static))
    }
}

impl From<sqlx::Error> for DdlError {
    fn from(error: sqlx::Error) -> Self {
        Self {
            code: "INTERNAL_SERVER_ERROR",
            message: error.to_string(),
            source: Some(error),
        }
    }
}

impl From<ColumnDefsError> for DdlError {
    fn from(error: ColumnDefsError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
            source: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::error::Error as _;

    #[test]
    fn ddl_error_displays_the_code_and_message() {
        let error = DdlError::enum_operation_failed("Cannot create enum with no values");

        assert_eq!(error.code(), "ENUM_OPERATION_FAILED");
        assert_eq!(
            error.to_string(),
            "ENUM_OPERATION_FAILED: Cannot create enum with no values"
        );
        assert!(error.source().is_none());
    }

    #[test]
    fn column_defs_errors_convert_with_their_code() {
        let error: DdlError = ColumnDefsError::UnsupportedFieldMetadataType {
            field_type: "ACTOR".to_string(),
        }
        .into();

        assert_eq!(error.code(), "UNSUPPORTED_FIELD_METADATA_TYPE");
    }
}
