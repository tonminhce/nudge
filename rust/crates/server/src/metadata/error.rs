#![allow(dead_code)]

pub const BUILDER_INTERNAL_SERVER_ERROR: &str = "BUILDER_INTERNAL_SERVER_ERROR";
pub const RUNNER_INTERNAL_SERVER_ERROR: &str = "RUNNER_INTERNAL_SERVER_ERROR";

#[derive(Debug)]
pub enum MetadataError {
    Build(crate::metadata::builders::OrchestratorFailureReport),
    Database(sqlx::Error),
    Engine(EngineError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    Validation { code: &'static str, message: String },
    Conflict { code: &'static str, message: String },
    NotFound { code: &'static str, message: String },
    Internal { code: &'static str, message: String },
}

impl EngineError {
    pub fn code(&self) -> &'static str {
        match self {
            EngineError::Validation { code, .. }
            | EngineError::Conflict { code, .. }
            | EngineError::NotFound { code, .. }
            | EngineError::Internal { code, .. } => code,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            EngineError::Validation { message, .. }
            | EngineError::Conflict { message, .. }
            | EngineError::NotFound { message, .. }
            | EngineError::Internal { message, .. } => message,
        }
    }

    pub fn validation(code: &'static str, message: impl Into<String>) -> Self {
        EngineError::Validation {
            code,
            message: message.into(),
        }
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        EngineError::Conflict {
            code,
            message: message.into(),
        }
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        EngineError::NotFound {
            code,
            message: message.into(),
        }
    }

    pub fn internal(code: &'static str, message: impl Into<String>) -> Self {
        EngineError::Internal {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for EngineError {}

impl std::fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataError::Build(report) => write!(
                formatter,
                "metadata build failed with {} validation errors",
                report.total_errors()
            ),
            MetadataError::Database(error) => write!(formatter, "database error: {error}"),
            MetadataError::Engine(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for MetadataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MetadataError::Database(error) => Some(error),
            MetadataError::Engine(error) => Some(error),
            MetadataError::Build(_) => None,
        }
    }
}

impl From<sqlx::Error> for MetadataError {
    fn from(error: sqlx::Error) -> Self {
        MetadataError::Database(error)
    }
}

impl From<EngineError> for MetadataError {
    fn from(error: EngineError) -> Self {
        MetadataError::Engine(error)
    }
}

impl MetadataError {
    pub fn code(&self) -> &'static str {
        match self {
            MetadataError::Build(_) => BUILDER_INTERNAL_SERVER_ERROR,
            MetadataError::Database(_) => RUNNER_INTERNAL_SERVER_ERROR,
            MetadataError::Engine(error) => match error.code() {
                "RUNNER_INTERNAL_SERVER_ERROR" => RUNNER_INTERNAL_SERVER_ERROR,
                _ => BUILDER_INTERNAL_SERVER_ERROR,
            },
        }
    }

    pub fn build(report: crate::metadata::builders::OrchestratorFailureReport) -> Self {
        MetadataError::Build(report)
    }
}

pub type MetadataResult<T, E = MetadataError> = std::result::Result<T, E>;
