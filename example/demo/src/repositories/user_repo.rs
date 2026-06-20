use crate::dtos::user::{CreateUser, UserResponse};
use crate::entities::user::{self, Entity as User};
use nova_boot::NovaError;
use nova_boot_sql::ReadWritePool;
use sea_orm::{ConnectionTrait, EntityTrait, Set};

pub struct UserRepository {
    pool: ReadWritePool,
}

impl UserRepository {
    pub fn new(pool: ReadWritePool) -> Self {
        Self { pool }
    }

    pub async fn list_users(&self) -> Result<Vec<UserResponse>, NovaError> {
        let users = self
            .pool
            .fetch_all(User::find())
            .await
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        Ok(users
            .into_iter()
            .map(|u| UserResponse {
                id: u.id,
                username: u.username,
                email: u.email,
            })
            .collect())
    }

    pub async fn insert_user(&self, payload: CreateUser) -> Result<UserResponse, NovaError> {
        let model = self
            .pool
            .insert_one(user::ActiveModel {
                id: sea_orm::NotSet,
                username: Set(payload.username),
                email: Set(payload.email),
            })
            .await
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        Ok(UserResponse {
            id: model.id,
            username: model.username,
            email: model.email,
        })
    }

    pub async fn db_status(&self) -> serde_json::Value {
        let db = self.pool.read_sync();
        let backend = db.get_database_backend();
        serde_json::json!({
            "connected": true,
            "backend": format!("{:?}", backend)
        })
    }
}
