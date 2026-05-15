use crate::app_state::{AppState, RuntimeConfig};
use nova_core::{
    Deserialize, Json, NovaError, NovaRequest, NovaResponse, NovaResult, Serialize,
    axum::Extension, axum::extract::Query, axum::http::StatusCode, get, post,
};

use nova_middleware::{
    ApiResponse, ApiVersion, ListResponse, PaginatedResponse, PaginationQuery, VersionedResponse,
};
use nova_middleware::{
    NovaValidate, ValidationErrors, max_length, min_length, required_string, validate_request,
};

use sea_orm::{ConnectionTrait, Statement};

fn demo_openapi_fragment() -> serde_json::Value {
    serde_json::json!({
        "tags": [
            {
                "name": "demo",
                "description": "Demo endpoints for Nova framework"
            }
        ]
    })
}

nova_core::inventory::submit! {
    nova_observability::OpenApiHook {
        name: "demo-tag-fragment",
        provider: demo_openapi_fragment,
    }
}

#[derive(Deserialize, NovaRequest)]
pub struct MessageReceived {
    #[serde(rename = "content", alias = "message")]
    pub content: String,
}

impl NovaValidate for MessageReceived {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        if let Some(err) = required_string("content", &self.content) {
            errors.push(err);
        }

        if let Some(err) = min_length("content", &self.content, 1) {
            errors.push(err);
        }

        if let Some(err) = max_length("content", &self.content, 512) {
            errors.push(err);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Serialize, NovaResponse)]
pub struct MessageSent {
    pub reply: String,
}

#[derive(Serialize, Clone, NovaResponse)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
}

#[derive(Deserialize, NovaRequest)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
}

impl NovaValidate for CreateUser {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        if let Some(err) = required_string("username", &self.username) {
            errors.push(err);
        }

        if let Some(err) = required_string("email", &self.email) {
            errors.push(err);
        }

        if let Some(err) = max_length("username", &self.username, 64) {
            errors.push(err);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Health check endpoint
#[get("/hello")]
pub async fn hello_world() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::with_status(
        StatusCode::OK,
        "Hello from Nova! 🚀",
    ))
}

/// Echo endpoint with proper error handling
#[post("/echo")]
pub async fn echo(
    Json(payload): Json<MessageReceived>,
) -> NovaResult<Json<ApiResponse<MessageSent>>> {
    validate_request(&payload)?;

    Ok(Json(ApiResponse::with_status(
        StatusCode::OK,
        MessageSent {
            reply: format!("Nova received: {}", payload.content),
        },
    )))
}

/// Get all users with error handling and structured response
#[get("/users")]
pub async fn get_users(
    Extension(pool): Extension<nova_sql::ReadWritePool>,
) -> NovaResult<Json<ApiResponse<ListResponse<UserResponse>>>> {
    let db = pool.read().await;
    let rows: Vec<sea_orm::QueryResult> = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT id, username, email FROM users",
            [],
        ))
        .await
        .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

    let mut users = Vec::new();

    for row in rows {
        let id: i32 = row
            .try_get_by_index(0)
            .map_err(|e| NovaError::DatabaseError(format!("Failed to parse ID: {}", e)))?;
        let username: String = row
            .try_get_by_index(1)
            .map_err(|e| NovaError::DatabaseError(format!("Failed to parse username: {}", e)))?;
        let email: String = row
            .try_get_by_index(2)
            .map_err(|e| NovaError::DatabaseError(format!("Failed to parse email: {}", e)))?;

        users.push(UserResponse {
            id,
            username,
            email,
        });
    }

    let total = users.len();
    let list_response = ListResponse::with_total(users, total);

    Ok(Json(ApiResponse::with_status(
        StatusCode::OK,
        list_response,
    )))
}

/// Get users with pagination helper metadata.
#[get("/users-paged")]
pub async fn get_users_paged(
    Query(pagination): Query<PaginationQuery>,
    Extension(pool): Extension<nova_sql::ReadWritePool>,
) -> NovaResult<Json<ApiResponse<PaginatedResponse<UserResponse>>>> {
    let db = pool.read().await;
    let rows: Vec<sea_orm::QueryResult> = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT id, username, email FROM users",
            [],
        ))
        .await
        .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

    let mut users = Vec::new();

    for row in rows {
        let id: i32 = row
            .try_get_by_index(0)
            .map_err(|e| NovaError::DatabaseError(format!("Failed to parse ID: {}", e)))?;
        let username: String = row
            .try_get_by_index(1)
            .map_err(|e| NovaError::DatabaseError(format!("Failed to parse username: {}", e)))?;
        let email: String = row
            .try_get_by_index(2)
            .map_err(|e| NovaError::DatabaseError(format!("Failed to parse email: {}", e)))?;

        users.push(UserResponse {
            id,
            username,
            email,
        });
    }

    let paged = PaginatedResponse::from_items(users, pagination);

    Ok(Json(ApiResponse::with_status(StatusCode::OK, paged)))
}

/// Demonstrates versioned response payloads.
#[get("/versioned-hello")]
pub async fn versioned_hello(
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> NovaResult<Json<ApiResponse<VersionedResponse<serde_json::Value>>>> {
    let version = query
        .get("v")
        .map(|v| v.parse::<ApiVersion>())
        .transpose()
        .map_err(|err| NovaError::BadRequest(err.to_string()))?
        .unwrap_or_default();

    let body = serde_json::json!({
        "message": "Hello from versioned endpoint",
        "path": "/versioned-hello",
    });

    Ok(Json(ApiResponse::with_status(
        StatusCode::OK,
        VersionedResponse::new(version, body),
    )))
}

/// Check database connection status
#[get("/db-status")]
pub async fn check_db(
    Extension(pool): Extension<nova_sql::ReadWritePool>,
) -> Json<ApiResponse<serde_json::Value>> {
    let db = pool.read().await;
    let backend = db.get_database_backend();
    let status = serde_json::json!({
        "connected": true,
        "backend": format!("{:?}", backend)
    });
    Json(ApiResponse::with_status(StatusCode::OK, status))
}

/// Returns the current runtime config loaded by the hot reloader.
#[get("/runtime-config")]
pub async fn runtime_config(
    Extension(state): Extension<AppState>,
) -> Json<ApiResponse<RuntimeConfig>> {
    let current = state.runtime_config.get().await;
    Json(ApiResponse::with_status(StatusCode::OK, current))
}

#[post("/users")]
pub async fn create_user(
    Extension(_pool): Extension<nova_sql::ReadWritePool>,
    Json(payload): Json<CreateUser>,
) -> NovaResult<Json<ApiResponse<UserResponse>>> {
    validate_request(&payload)?;

    // Perform a write against the primary database using raw SQL to avoid
    // needing ActiveModel/Entity trait bounds in this example.
    let db = _pool.write();

    let insert = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        "INSERT INTO users (username, email) VALUES (?, ?)",
        vec![
            payload.username.clone().into(),
            payload.email.clone().into(),
        ],
    );

    db.execute(insert)
        .await
        .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

    let row = db
        .query_one(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT id, username, email FROM users WHERE rowid = last_insert_rowid()",
            vec![],
        ))
        .await
        .map_err(|e| NovaError::DatabaseError(e.to_string()))?
        .ok_or_else(|| NovaError::DatabaseError("Inserted row not found".to_string()))?;

    let id: i32 = row
        .try_get_by_index(0)
        .map_err(|e| NovaError::DatabaseError(format!("Failed to parse ID: {}", e)))?;
    let username: String = row
        .try_get_by_index(1)
        .map_err(|e| NovaError::DatabaseError(format!("Failed to parse username: {}", e)))?;
    let email: String = row
        .try_get_by_index(2)
        .map_err(|e| NovaError::DatabaseError(format!("Failed to parse email: {}", e)))?;

    Ok(Json(ApiResponse::with_status(
        StatusCode::CREATED,
        UserResponse {
            id,
            username,
            email,
        },
    )))
}
