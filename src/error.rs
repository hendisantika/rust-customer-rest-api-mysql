use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use validator::ValidationErrors;

/// Every failure that can be turned into an HTTP response.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("customer {0} was not found")]
    NotFound(u64),

    #[error("the request failed validation")]
    Validation(#[from] ValidationErrors),

    #[error("a customer with email '{0}' already exists")]
    DuplicateEmail(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Problem payload returned for every non-2xx response.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: u16,
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::DuplicateEmail(_) => StatusCode::CONFLICT,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();

        // Database failures may carry connection details, so they are logged
        // server side and reported to the client as a generic message.
        let (message, details) = match &self {
            Self::Database(error) => {
                tracing::error!(%error, "database error");
                ("an internal error occurred".to_owned(), None)
            }
            Self::Validation(errors) => (self.to_string(), serde_json::to_value(errors).ok()),
            _ => (self.to_string(), None),
        };

        let body = ErrorResponse {
            status: status.as_u16(),
            error: status.canonical_reason().unwrap_or("Error").to_owned(),
            message,
            details,
        };

        (status, Json(body)).into_response()
    }
}
