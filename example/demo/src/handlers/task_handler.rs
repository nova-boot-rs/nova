use crate::dtos::create_task::CreateTask;
use crate::dtos::task_response::TaskResponse;
use crate::services::task_service::TaskService;
use nova_boot::{Json, NovaResult, axum::extract::Path, axum::http::StatusCode, get, post};
use nova_boot_middleware::{ApiResponse, validate_request};
use nova_boot_sql::NovaDb;

// Task endpoints show the full request flow:
// HTTP extractor -> request validation -> service layer -> repository layer.
#[get("/tenants/{tenant_id}/tasks")]
pub async fn list_tasks(
    Path(tenant_id): Path<String>,
    NovaDb(pool): NovaDb,
) -> NovaResult<Json<ApiResponse<Vec<TaskResponse>>>> {
    // The extractor gives us a live DB pool without manual app-state plumbing.
    let service = TaskService::new(pool);
    let tasks = service.list_for_tenant(&tenant_id).await?;

    Ok(Json(ApiResponse::with_status(StatusCode::OK, tasks)))
}

#[post("/tenants/{tenant_id}/tasks")]
pub async fn create_task(
    Path(tenant_id): Path<String>,
    NovaDb(pool): NovaDb,
    Json(payload): Json<CreateTask>,
) -> NovaResult<Json<ApiResponse<TaskResponse>>> {
    // Validation is handled before the repository call so the demo shows
    // Nova's request-validation helpers in a realistic write path.
    validate_request(&payload)?;

    let service = TaskService::new(pool);
    let created = service.create_for_tenant(&tenant_id, payload).await?;

    Ok(Json(ApiResponse::with_status(StatusCode::CREATED, created)))
}
