use nova_core::{
    ApiResponse, Deserialize, Json, ListResponse, NovaError, NovaResult, Serialize,
    axum::Extension, axum::http::StatusCode, get, post,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

#[derive(Deserialize)]
pub struct MessageReceived {
    #[serde(rename = "content", alias = "message")]
    pub content: String,
}

#[derive(Serialize)]
pub struct MessageSent {
    pub reply: String,
}

#[derive(Serialize, Clone)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
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
    if payload.content.is_empty() {
        return Err(NovaError::Custom {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            error: "ValidationError".to_string(),
            message: "Message cannot be empty".to_string(),
        });
    }

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
    Extension(db): Extension<DatabaseConnection>,
) -> NovaResult<Json<ApiResponse<ListResponse<UserResponse>>>> {
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

/// Check database connection status
#[get("/db-status")]
pub async fn check_db(
    Extension(db): Extension<DatabaseConnection>,
) -> Json<ApiResponse<serde_json::Value>> {
    let backend = db.get_database_backend();
    let status = serde_json::json!({
        "connected": true,
        "backend": format!("{:?}", backend)
    });
    Json(ApiResponse::with_status(StatusCode::OK, status))
}

/// Health check endpoint
#[get("/health")]
pub async fn health_check() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::with_status(
        StatusCode::OK,
        serde_json::json!({"status": "healthy"}),
    ))
}
