use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::env;
use std::fmt;

/// Standardized error response for Nova API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<String>,
}

/// Nova framework error types
#[derive(Debug)]
pub enum NovaError {
    /// Database connection or query errors
    DatabaseError(String),

    /// Validation failed
    ValidationError(String),

    /// Authentication failed
    AuthenticationError(String),

    /// Authorization failed (forbidden)
    AuthorizationError(String),

    /// Resource not found
    NotFound(String),

    /// Conflict error (e.g., duplicate entry)
    Conflict(String),

    /// Internal server error
    InternalError(String),

    /// Bad request / Invalid input
    BadRequest(String),

    /// Custom error with status code
    Custom {
        status: StatusCode,
        error: String,
        message: String,
    },
}

impl fmt::Display for NovaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NovaError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            NovaError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            NovaError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            NovaError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            NovaError::NotFound(msg) => write!(f, "Not found: {}", msg),
            NovaError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            NovaError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            NovaError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            NovaError::Custom { message, .. } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for NovaError {}

/// Discovery subsystem errors.
#[derive(Debug, Clone)]
pub enum DiscoveryError {
    NotFound(String),
    Backend(String),
    ConnectionFailed(String),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Discovery not found: {}", msg),
            Self::Backend(msg) => write!(f, "Discovery backend error: {}", msg),
            Self::ConnectionFailed(msg) => write!(f, "Discovery connection failed: {}", msg),
        }
    }
}

impl std::error::Error for DiscoveryError {}

impl NovaError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            NovaError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            NovaError::ValidationError(_) => StatusCode::BAD_REQUEST,
            NovaError::AuthenticationError(_) => StatusCode::UNAUTHORIZED,
            NovaError::AuthorizationError(_) => StatusCode::FORBIDDEN,
            NovaError::NotFound(_) => StatusCode::NOT_FOUND,
            NovaError::Conflict(_) => StatusCode::CONFLICT,
            NovaError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            NovaError::BadRequest(_) => StatusCode::BAD_REQUEST,
            NovaError::Custom { status, .. } => *status,
        }
    }

    /// Get the error type name
    pub fn error_type(&self) -> &str {
        match self {
            NovaError::DatabaseError(_) => "DatabaseError",
            NovaError::ValidationError(_) => "ValidationError",
            NovaError::AuthenticationError(_) => "AuthenticationError",
            NovaError::AuthorizationError(_) => "AuthorizationError",
            NovaError::NotFound(_) => "NotFound",
            NovaError::Conflict(_) => "Conflict",
            NovaError::InternalError(_) => "InternalError",
            NovaError::BadRequest(_) => "BadRequest",
            NovaError::Custom { error, .. } => error,
        }
    }

    /// Get the message
    pub fn message(&self) -> String {
        match self {
            NovaError::Custom { message, .. } => message.clone(),
            err => err.to_string(),
        }
    }
}

impl IntoResponse for NovaError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        // If `NOVA_DEBUG=true` is set, include a captured backtrace (which
        // will contain file and line information when available) in the
        // `details` field of the JSON error response. Otherwise omit details
        // to avoid leaking internal information in production.
        let details = match env::var("NOVA_DEBUG") {
            Ok(val) if val.eq_ignore_ascii_case("true") || val == "1" => {
                Some(format!("{:?}", Backtrace::capture()))
            }
            _ => None,
        };

        let body = Json(ErrorResponse {
            error: self.error_type().to_string(),
            message: self.message(),
            details,
        });

        (status, body).into_response()
    }
}

/// Convenience type alias for Results in Nova applications
pub type NovaResult<T> = Result<T, NovaError>;

// Common From implementations for easy error conversion
impl From<std::io::Error> for NovaError {
    fn from(err: std::io::Error) -> Self {
        NovaError::InternalError(err.to_string())
    }
}

impl From<serde_json::Error> for NovaError {
    fn from(err: serde_json::Error) -> Self {
        NovaError::BadRequest(format!("Invalid JSON: {}", err))
    }
}

impl From<nova_resilience_store::ResilienceError> for NovaError {
    fn from(err: nova_resilience_store::ResilienceError) -> Self {
        NovaError::InternalError(err.to_string())
    }
}

#[cfg(feature = "database")]
impl From<sea_orm::DbErr> for NovaError {
    fn from(err: sea_orm::DbErr) -> Self {
        NovaError::DatabaseError(err.to_string())
    }
}
