use crate::dtos::task_response::TaskResponse;
use crate::entities::task::{self, Column as TaskColumn, Entity as Task};
use nova_boot::NovaError;
use nova_boot_sql::ReadWritePool;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

pub struct TaskRepository {
    pool: ReadWritePool,
}

impl TaskRepository {
    pub fn new(pool: ReadWritePool) -> Self {
        Self { pool }
    }

    pub async fn list_for_tenant(&self, tenant_id: &str) -> Result<Vec<TaskResponse>, NovaError> {
        let tasks = self
            .pool
            .fetch_all(
                Task::find()
                    .filter(TaskColumn::TenantId.eq(tenant_id))
                    .order_by(TaskColumn::CreatedAt, sea_orm::Order::Desc),
            )
            .await
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        Ok(tasks
            .into_iter()
            .map(|t| TaskResponse {
                id: t.id,
                title: t.title,
            })
            .collect())
    }

    pub async fn create_task(
        &self,
        tenant_id: &str,
        title: &str,
        payload: Option<&serde_json::Value>,
    ) -> Result<TaskResponse, NovaError> {
        let payload_str = payload
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        let model = self
            .pool
            .insert_one(task::ActiveModel {
                id: sea_orm::NotSet,
                tenant_id: Set(tenant_id.to_owned()),
                title: Set(title.to_owned()),
                payload: Set(payload_str),
                status: Set(Some("active".to_owned())),
                created_at: Set(None),
            })
            .await
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        Ok(TaskResponse {
            id: model.id,
            title: model.title,
        })
    }
}
