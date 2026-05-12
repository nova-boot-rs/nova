use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// Standardized API response wrapper for consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    #[serde(skip)]
    pub status: u16,
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response with data
    pub fn ok(data: T) -> Self {
        ApiResponse {
            status: StatusCode::OK.as_u16(),
            success: true,
            data: Some(data),
            message: None,
        }
    }

    /// Create a successful response with a custom HTTP status code
    pub fn with_status(status: StatusCode, data: T) -> Self {
        ApiResponse {
            status: status.as_u16(),
            success: status.is_success(),
            data: Some(data),
            message: None,
        }
    }

    /// Create a successful response with data and a message
    pub fn ok_with_message(data: T, message: impl Into<String>) -> Self {
        ApiResponse {
            status: StatusCode::OK.as_u16(),
            success: true,
            data: Some(data),
            message: Some(message.into()),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::OK)
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = self.status_code();
        (status, Json(self)).into_response()
    }
}

/// Empty response for operations that don't return data (e.g., DELETE)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyResponse;

impl IntoResponse for EmptyResponse {
    fn into_response(self) -> Response {
        StatusCode::NO_CONTENT.into_response()
    }
}

/// List response for paginated or multiple items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T: Serialize> {
    pub items: Vec<T>,
    pub count: usize,
    pub total: Option<usize>, // Total items available (for pagination)
}

impl<T: Serialize> ListResponse<T> {
    pub fn new(items: Vec<T>) -> Self {
        let count = items.len();
        ListResponse {
            items,
            count,
            total: None,
        }
    }

    pub fn with_total(items: Vec<T>, total: usize) -> Self {
        let count = items.len();
        ListResponse {
            items,
            count,
            total: Some(total),
        }
    }
}

impl<T: Serialize> IntoResponse for ListResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
