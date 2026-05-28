//! Task service — business rules for tenant-scoped task operations.

use crate::dtos::{create_task::CreateTask, task_response::TaskResponse};
use crate::repositories::TaskRepository;
use nova_boot::NovaError;
use nova_boot_sql::ReadWritePool;

// The service layer keeps business rules in one place even when the repo is
// simple today; that makes the demo easier to extend later.
pub struct TaskService {
    repo: TaskRepository,
}

impl TaskService {
    pub fn new(pool: ReadWritePool) -> Self {
        Self {
            repo: TaskRepository::new(pool),
        }
    }

    pub async fn list_for_tenant(&self, tenant_id: &str) -> Result<Vec<TaskResponse>, NovaError> {
        self.repo.list_for_tenant(tenant_id).await
    }

    pub async fn create_for_tenant(
        &self,
        tenant_id: &str,
        payload: CreateTask,
    ) -> Result<TaskResponse, NovaError> {
        // Map request DTOs to repository inputs here so handlers stay thin.
        self.repo
            .create_task(tenant_id, &payload.title, payload.payload.as_ref())
            .await
    }
}
