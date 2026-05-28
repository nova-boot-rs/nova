use crate::dtos::user::{CreateUser, UserResponse};
use crate::repositories::UserRepository;
use nova_boot::{Json, NovaResult, axum::extract::Query, axum::http::StatusCode, get, post};
use nova_boot_middleware::{
    ApiResponse, ListResponse, PaginatedResponse, PaginationQuery, validate_request,
};
use nova_boot_sql::NovaDb;

// User endpoints demonstrate the simplest happy-path CRUD flow in Nova.
#[get("/users")]
pub async fn get_users(
    NovaDb(pool): NovaDb,
) -> NovaResult<Json<ApiResponse<ListResponse<UserResponse>>>> {
    // The repository hides the raw SQL so the handler stays focused on HTTP.
    let repo = UserRepository::new(pool);
    let users = repo.list_users().await?;
    let total = users.len();
    Ok(Json(ApiResponse::with_status(
        StatusCode::OK,
        ListResponse::with_total(users, total),
    )))
}

#[get("/users-paged")]
pub async fn get_users_paged(
    Query(pagination): Query<PaginationQuery>,
    NovaDb(pool): NovaDb,
) -> NovaResult<Json<ApiResponse<PaginatedResponse<UserResponse>>>> {
    let repo = UserRepository::new(pool);
    let users = repo.list_users().await?;
    Ok(Json(ApiResponse::with_status(
        StatusCode::OK,
        PaginatedResponse::from_items(users, pagination),
    )))
}

#[get("/db-status")]
pub async fn check_db(NovaDb(pool): NovaDb) -> NovaResult<Json<ApiResponse<serde_json::Value>>> {
    // This endpoint is useful for the demo because it proves the extractor
    // and plugin wiring are working before touching business data.
    let repo = UserRepository::new(pool);
    let status = repo.db_status().await;
    Ok(Json(ApiResponse::with_status(StatusCode::OK, status)))
}

#[post("/users")]
pub async fn create_user(
    NovaDb(pool): NovaDb,
    Json(payload): Json<CreateUser>,
) -> NovaResult<Json<ApiResponse<UserResponse>>> {
    // Request validation stays at the edge, right before the write operation.
    validate_request(&payload)?;
    let repo = UserRepository::new(pool);
    let created = repo.insert_user(payload).await?;
    Ok(Json(ApiResponse::with_status(StatusCode::CREATED, created)))
}
