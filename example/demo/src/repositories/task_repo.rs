//! Task repository — SeaORM-backed, tenant-scoped queries.
//!
//! Uses `nova-boot-sql`'s `ReadWritePool` and `TenantScope` utilities together
//! with `sea_query` to keep tenant filtering explicit in the generated SQL.

use crate::dtos::task_response::TaskResponse;
use nova_boot::NovaError;
use nova_boot_sql::{ConnectionTrait, DbBackend, ReadWritePool, Statement, Tenant, TenantScope};
use sea_query::{Alias, Expr, MysqlQueryBuilder, PostgresQueryBuilder, Query, SqliteQueryBuilder};
use serde_json::Value as JsonValue;

pub struct TaskRepository {
    pool: ReadWritePool,
}

impl TaskRepository {
    pub fn new(pool: ReadWritePool) -> Self {
        Self { pool }
    }

    // Build a tenant-aware scope so the demo can show multi-tenant filtering
    // without hard-coding tenant checks into every query.
    fn scope_for_tenant(&self, tenant_id: &str, db: sea_orm::DatabaseConnection) -> TenantScope {
        TenantScope::new(db, Tenant::new(tenant_id))
    }

    // SeaQuery builds SQL in a backend-aware way while the scope injects the
    // tenant filter for the `tasks` table.
    fn select_sql(scope: &TenantScope, backend: DbBackend) -> Result<String, NovaError> {
        let mut stmt = Query::select();
        stmt.from(Alias::new("tasks"));
        stmt.expr(Expr::col((Alias::new("tasks"), Alias::new("id"))));
        stmt.expr(Expr::col((Alias::new("tasks"), Alias::new("title"))));
        stmt.expr(Expr::col((Alias::new("tasks"), Alias::new("created_at"))));
        stmt.order_by(
            (Alias::new("tasks"), Alias::new("created_at")),
            sea_query::Order::Desc,
        );

        scope
            .apply_select_scope("tasks", &mut stmt)
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        let sql = match backend {
            DbBackend::Sqlite => stmt.to_string(SqliteQueryBuilder),
            DbBackend::Postgres => stmt.to_string(PostgresQueryBuilder),
            _ => stmt.to_string(MysqlQueryBuilder),
        };

        Ok(sql)
    }

    /// List tasks for a tenant.
    pub async fn list_for_tenant(&self, tenant_id: &str) -> Result<Vec<TaskResponse>, NovaError> {
        // Read operations use the read pool, then the query is scoped to the
        // current tenant before execution.
        let db = self.pool.read().await;
        let backend = db.get_database_backend();
        let scope = self.scope_for_tenant(tenant_id, db.clone());
        let sql = Self::select_sql(&scope, backend)?;

        let rows = db
            .query_all(Statement::from_string(backend, sql))
            .await
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id: i64 = row
                .try_get("", "id")
                .map_err(|e| NovaError::DatabaseError(e.to_string()))?;
            let title: String = row
                .try_get("", "title")
                .map_err(|e| NovaError::DatabaseError(e.to_string()))?;
            out.push(TaskResponse { id, title });
        }

        Ok(out)
    }

    /// Create a new task under a tenant.
    pub async fn create_task(
        &self,
        tenant_id: &str,
        title: &str,
        payload: Option<&JsonValue>,
    ) -> Result<TaskResponse, NovaError> {
        // Writes use the writer connection, and the tenant column is resolved
        // from the scope so inserts stay consistent with the tenant model.
        let db = self.pool.write();
        let backend = db.get_database_backend();
        let scope = self.scope_for_tenant(tenant_id, db.clone());
        let tenant_column = scope
            .tenant_column_for_table("tasks")
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;
        let payload_str = payload
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        let mut insert = Query::insert();
        insert.into_table(Alias::new("tasks"));
        insert.columns([
            Alias::new(tenant_column),
            Alias::new("title"),
            Alias::new("payload"),
        ]);
        insert.values_panic([tenant_id.into(), title.into(), payload_str.into()]);

        let insert_sql = match backend {
            DbBackend::Sqlite => insert.to_string(SqliteQueryBuilder),
            DbBackend::Postgres => insert.to_string(PostgresQueryBuilder),
            _ => insert.to_string(MysqlQueryBuilder),
        };

        db.execute(Statement::from_string(backend, insert_sql))
            .await
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        // Return the inserted row so the API response can echo the created task.
        let row = db
            .query_one(Statement::from_string(
                backend,
                "SELECT id, title FROM tasks WHERE rowid = last_insert_rowid()".to_string(),
            ))
            .await
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?
            .ok_or_else(|| NovaError::DatabaseError("Inserted row not found".to_string()))?;

        let id: i64 = row
            .try_get("", "id")
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;
        let title: String = row
            .try_get("", "title")
            .map_err(|e| NovaError::DatabaseError(e.to_string()))?;

        Ok(TaskResponse { id, title })
    }
}
