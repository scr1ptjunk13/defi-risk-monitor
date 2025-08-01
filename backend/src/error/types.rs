use std::fmt;
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

#[derive(Debug, Clone)]
pub enum AppError {
    DatabaseError(String),
    BlockchainError(String),
    ConfigError(String),
    ValidationError(String),
    NotFound(String),
    AlertError(String),
    AuthenticationError(String),
    AuthorizationError(String),
    RateLimitError(String),
    SecurityError(String),
    ExternalServiceError(String),
    ExternalApiError(String),
    UnsupportedChain(i32),
    InternalError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::BlockchainError(msg) => write!(f, "Blockchain error: {}", msg),
            AppError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::AlertError(msg) => write!(f, "Alert error: {}", msg),
            AppError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            AppError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            AppError::RateLimitError(msg) => write!(f, "Rate limit error: {}", msg),
            AppError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            AppError::ExternalServiceError(msg) => write!(f, "External service error: {}", msg),
            AppError::ExternalApiError(msg) => write!(f, "External API error: {}", msg),
            AppError::UnsupportedChain(chain_id) => write!(f, "Unsupported chain: {}", chain_id),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::AuthenticationError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::AuthorizationError(_) => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::RateLimitError(_) => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        AppError::ConfigError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::InternalError(format!("HTTP request error: {}", err))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InternalError(format!("JSON serialization error: {}", err))
    }
}

impl From<prometheus::Error> for AppError {
    fn from(err: prometheus::Error) -> Self {
        AppError::InternalError(format!("Prometheus metrics error: {}", err))
    }
}
