use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiVersion {
    #[default]
    V1,
    V2,
}

impl ApiVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiVersion::V1 => "v1",
            ApiVersion::V2 => "v2",
        }
    }
}

impl FromStr for ApiVersion {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "v1" | "1" => Ok(ApiVersion::V1),
            "v2" | "2" => Ok(ApiVersion::V2),
            _ => Err(format!("unsupported api version: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedResponse<T: Serialize> {
    pub version: String,
    pub data: T,
}

impl<T: Serialize> VersionedResponse<T> {
    pub fn new(version: ApiVersion, data: T) -> Self {
        Self {
            version: version.as_str().to_string(),
            data,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(20),
        }
    }
}

impl PaginationQuery {
    pub fn normalized(&self) -> (usize, usize) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).clamp(1, 100);
        (page, per_page)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub page: usize,
    pub per_page: usize,
    pub total: usize,
    pub total_pages: usize,
    pub has_next: bool,
    pub has_prev: bool,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn from_items(items: Vec<T>, query: PaginationQuery) -> Self {
        let total = items.len();
        let (page, per_page) = query.normalized();
        let offset = (page - 1) * per_page;

        let page_items = if offset >= total {
            Vec::new()
        } else {
            items.into_iter().skip(offset).take(per_page).collect()
        };

        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(per_page)
        };

        Self {
            items: page_items,
            page,
            per_page,
            total,
            total_pages,
            has_next: total_pages > 0 && page < total_pages,
            has_prev: page > 1 && total_pages > 0,
        }
    }
}
