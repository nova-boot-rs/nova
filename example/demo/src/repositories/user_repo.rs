use crate::dtos::user::{CreateUser, UserResponse};
use nova_boot::NovaError;
use nova_boot_sql::{ConnectionTrait, ReadWritePool, Statement};

// The user repository keeps the demo's basic CRUD example separate from HTTP
// concerns while still showing direct SQL access through the plugin pool.
pub struct UserRepository {
    pool: ReadWritePool,
}

impl UserRepository {
    pub fn new(pool: ReadWritePool) -> Self {
        Self { pool }
    }

    pub async fn list_users(&self) -> Result<Vec<UserResponse>, NovaError> {
        // Read from the shared pool and execute plain SQL for a minimal demo.
        let db = self.pool.read().await;
        let rows: Vec<sea_orm::QueryResult> = db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Sqlite,
                "SELECT id, username, email FROM users",
                [],
            ))
            .await
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i32 = row
                .try_get_by_index(0)
                .map_err(|e| NovaError::DatabaseError(format!("Failed to parse ID: {}", e)))?;
            let username: String = row.try_get_by_index(1).map_err(|e| {
                NovaError::DatabaseError(format!("Failed to parse username: {}", e))
            })?;
            let email: String = row
                .try_get_by_index(2)
                .map_err(|e| NovaError::DatabaseError(format!("Failed to parse email: {}", e)))?;

            users.push(UserResponse {
                id,
                username,
                email,
            });
        }

        Ok(users)
    }

    pub async fn insert_user(&self, payload: CreateUser) -> Result<UserResponse, NovaError> {
        // The demo writes a row, then reads the inserted record back to build
        // a strongly typed response object.
        let db = self.pool.write();

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

        Ok(UserResponse {
            id,
            username,
            email,
        })
    }

    pub async fn db_status(&self) -> serde_json::Value {
        // Handy smoke-test endpoint: it proves the DB pool is wired correctly.
        let db = self.pool.read().await;
        let backend = db.get_database_backend();
        serde_json::json!({
            "connected": true,
            "backend": format!("{:?}", backend)
        })
    }
}
