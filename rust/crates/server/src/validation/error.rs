use crate::query::runner::RunnerError;

pub const INVALID_ARGS_DATA: &str = "INVALID_ARGS_DATA";

pub const STANDARD_ERROR_MESSAGE: &str = "An error occurred.";

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub code: &'static str,
    pub message: String,
    pub user_friendly_message: String,
}

impl ValidationError {
    pub fn new(message: impl Into<String>, user_friendly_message: impl Into<String>) -> Self {
        Self {
            code: INVALID_ARGS_DATA,
            message: message.into(),
            user_friendly_message: user_friendly_message.into(),
        }
    }

    pub fn standard(message: impl Into<String>) -> Self {
        Self::new(message, STANDARD_ERROR_MESSAGE)
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn user_friendly_message(&self) -> &str {
        &self.user_friendly_message
    }

    pub fn into_runner_error(self) -> RunnerError {
        RunnerError::new(self.code, self.message)
    }
}

impl From<ValidationError> for RunnerError {
    fn from(error: ValidationError) -> Self {
        error.into_runner_error()
    }
}

pub type ValidationResult<T> = Result<T, ValidationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_into_a_runner_error_carrying_the_invalid_args_data_code() {
        let error = ValidationError::new("boom", "An error occurred.");
        let runner_error: RunnerError = error.into();

        assert_eq!(runner_error.code(), "INVALID_ARGS_DATA");
        assert_eq!(runner_error.message(), "boom");
    }
}
