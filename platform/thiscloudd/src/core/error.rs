use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

/// Application-level error type used by all HTTP handler modules.
///
/// Variants map directly to HTTP status codes so handler code never has to
/// guess the right status based on a formatted string.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Validation(String),

    #[error("{0}")]
    Conflict(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for AppError {
    /// Convert an [`anyhow::Error`] into a typed [`AppError`].
    ///
    /// This is the single point where message-based classification happens.
    /// `IntoResponse` then maps purely on the enum variant — no string
    /// matching at response time.
    fn from(err: anyhow::Error) -> Self {
        let msg = err.to_string();
        let lower = msg.to_lowercase();
        if lower.contains("not found") {
            Self::NotFound(msg)
        } else if lower.contains("already exists") || lower.contains("exceeds quota") {
            Self::Conflict(msg)
        } else {
            Self::Internal(msg)
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = serde_json::json!({ "error": self.to_string() });
        (status, Json(body)).into_response()
    }
}
