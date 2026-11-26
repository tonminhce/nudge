use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug)]
pub struct AuthError {
    pub status: u16,
    pub code: &'static str,
    pub messages: Vec<String>,
}

impl AuthError {
    pub fn new(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            messages: vec![message.into()],
        }
    }

    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(401, "UNAUTHENTICATED", message)
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(400, "INVALID_INPUT", message)
    }

    pub fn user_already_exists(message: impl Into<String>) -> Self {
        Self::new(403, "USER_ALREADY_EXISTS", message)
    }

    pub fn user_not_found(message: impl Into<String>) -> Self {
        Self::new(401, "USER_NOT_FOUND", message)
    }

    pub fn workspace_not_found(message: impl Into<String>) -> Self {
        Self::new(401, "WORKSPACE_NOT_FOUND", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403, "FORBIDDEN_EXCEPTION", message)
    }

    pub fn invalid_jwt_token_type(message: impl Into<String>) -> Self {
        Self::new(403, "INVALID_JWT_TOKEN_TYPE", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(500, "INTERNAL_SERVER_ERROR", message)
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.messages.join(", "))
    }
}

impl From<nudge_core::Error> for AuthError {
    fn from(error: nudge_core::Error) -> Self {
        tracing::error!(%error, "unexpected error");

        AuthError::internal(error.to_string())
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database error");

        AuthError::internal(error.to_string())
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            status_code: u16,
            error: &'a str,
            messages: &'a [String],
        }

        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (
            status,
            Json(Body {
                status_code: self.status,
                error: self.code,
                messages: &self.messages,
            }),
        )
            .into_response()
    }
}

pub type AuthResult<T, E = AuthError> = std::result::Result<T, E>;
