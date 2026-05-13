use nova_core::{NovaPlugin, async_trait, axum::Extension, axum::Router};
pub use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Schema, Statement,
};
pub use sea_orm_migration::prelude::*;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Type alias for database sync task closures
type SyncTask = Box<
    dyn for<'a> Fn(
            &'a NovaSql,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>
        + Send
        + Sync,
>;

pub struct NovaSql {
    pub db: DatabaseConnection,
    pub allow_drop: bool,
    sync_tasks: Vec<SyncTask>,
    cache_store: Option<Arc<dyn QueryCacheStore>>,
}

#[async_trait]
pub trait QueryCacheStore: Send + Sync {
    async fn get(&self, key: &str) -> Option<String>;
    async fn set(&self, key: &str, value: String, ttl: Duration);
    async fn del(&self, key: &str);
}

#[derive(Clone, Default)]
pub struct InMemoryQueryCache {
    inner: Arc<Mutex<HashMap<String, (String, Instant)>>>,
}

#[async_trait]
impl QueryCacheStore for InMemoryQueryCache {
    async fn get(&self, key: &str) -> Option<String> {
        let mut map = self.inner.lock().await;
        if let Some((value, expires_at)) = map.get(key)
            && Instant::now() <= *expires_at
        {
            return Some(value.clone());
        }

        map.remove(key);
        None
    }

    async fn set(&self, key: &str, value: String, ttl: Duration) {
        let expires_at = Instant::now() + ttl;
        self.inner
            .lock()
            .await
            .insert(key.to_string(), (value, expires_at));
    }

    async fn del(&self, key: &str) {
        self.inner.lock().await.remove(key);
    }
}

impl NovaSql {
    pub async fn connect(url: &str, allow_drop: bool) -> Self {
        let db = Database::connect(url)
            .await
            .expect("Failed to connect to the database");
        Self {
            db,
            allow_drop,
            sync_tasks: Vec::new(),
            cache_store: None,
        }
    }

    pub fn with_cache_store(mut self, cache_store: Arc<dyn QueryCacheStore>) -> Self {
        self.cache_store = Some(cache_store);
        self
    }

    pub async fn cached_json<F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        fetcher: F,
    ) -> Result<JsonValue, DbErr>
    where
        F: FnOnce(&DatabaseConnection) -> Fut,
        Fut: std::future::Future<Output = Result<JsonValue, DbErr>>,
    {
        if let Some(cache) = &self.cache_store
            && let Some(raw) = cache.get(key).await
            && let Ok(value) = serde_json::from_str::<JsonValue>(&raw)
        {
            return Ok(value);
        }

        let value = fetcher(&self.db).await?;

        if let Some(cache) = &self.cache_store {
            let raw = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
            cache.set(key, raw, ttl).await;
        }

        Ok(value)
    }

    pub async fn invalidate_cache(&self, key: &str) {
        if let Some(cache) = &self.cache_store {
            cache.del(key).await;
        }
    }

    pub fn add_entity<E>(mut self) -> Self
    where
        E: EntityTrait + 'static,
    {
        self.sync_tasks
            .push(Box::new(|sql| Box::pin(sql.sync_entity::<E>())));
        self
    }

    pub async fn sync_entity<E>(&self)
    where
        E: EntityTrait,
    {
        let entity = E::default();
        let table_name = entity.table_name().to_string();
        let builder = self.db.get_database_backend();
        let schema = Schema::new(builder);

        let existing_columns = self.get_table_columns(&table_name).await;

        if existing_columns.is_empty() {
            // ... (Keep your existing Create Table logic)
        } else {
            let table_create_stmt = schema.create_table_from_entity(entity);
            let model_columns: Vec<String> = table_create_stmt
                .get_columns()
                .iter()
                .map(|c| c.get_column_name().to_string())
                .collect();

            // 1. ADD missing columns (Code -> DB)
            for column in table_create_stmt.get_columns() {
                let col_name = column.get_column_name().to_string();
                if !existing_columns.contains(&col_name) {
                    let alter_stmt = builder.build(
                        &sea_query::Table::alter()
                            .table(sea_query::Alias::new(&table_name))
                            .add_column(column.clone())
                            .to_owned(),
                    );
                    self.db.execute(alter_stmt).await.ok();
                }
            }

            if self.allow_drop {
                for db_col in existing_columns {
                    if !model_columns.contains(&db_col) {
                        println!(
                            "🗑️ Dropping unused column '{}' from '{}'",
                            db_col, table_name
                        );

                        let drop_stmt = builder.build(
                            &sea_query::Table::alter()
                                .table(sea_query::Alias::new(&table_name))
                                .drop_column(sea_query::Alias::new(&db_col))
                                .to_owned(),
                        );

                        // Note: SQLite doesn't support DROP COLUMN in older versions.
                        // Sea-ORM/Sea-Query handles the abstraction for modern SQLite.
                        if let Err(e) = self.db.execute(drop_stmt).await {
                            println!("⚠️ Could not drop column {}: {}", db_col, e);
                        }
                    }
                }
            }
        }
    }

    /// Helper to fetch column names based on the database type
    async fn get_table_columns(&self, table_name: &str) -> Vec<String> {
        let mut columns = Vec::new();

        match self.db.get_database_backend() {
            DbBackend::Sqlite => {
                let sql = format!("PRAGMA table_info('{}')", table_name);
                let res = self
                    .db
                    .query_all(Statement::from_string(DbBackend::Sqlite, sql))
                    .await
                    .unwrap();
                for row in res {
                    let name: String = row.try_get("", "name").unwrap_or_default();
                    columns.push(name);
                }
            }
            DbBackend::Postgres => {
                let sql =
                    "SELECT column_name FROM information_schema.columns WHERE table_name = $1";
                let res = self
                    .db
                    .query_all(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        sql,
                        vec![table_name.into()],
                    ))
                    .await
                    .unwrap();
                for row in res {
                    let name: String = row.try_get("", "column_name").unwrap_or_default();
                    columns.push(name);
                }
            }
            _ => println!("⚠️ Database backend not supported for auto-sync yet."),
        }
        columns
    }

    pub async fn run_migrations<M>(&self)
    where
        M: MigratorTrait,
    {
        M::up(&self.db, None).await.expect("Failed migrations");
    }
}

#[async_trait]
impl NovaPlugin for NovaSql {
    fn name(&self) -> &'static str {
        "NovaSql (Relational Engine)"
    }

    async fn on_init(&self) {
        println!("🗄️ Initializing SQL Plugin...");
        for task in &self.sync_tasks {
            task(self).await;
        }
    }

    fn extend_router(&self, router: Router) -> Router {
        router.layer(Extension(self.db.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_query_cache_expires_entries() {
        let cache = InMemoryQueryCache::default();

        cache
            .set(
                "users:1",
                "{\"id\":1}".to_string(),
                Duration::from_millis(10),
            )
            .await;

        let hit = cache.get("users:1").await;
        assert!(hit.is_some());

        tokio::time::sleep(Duration::from_millis(20)).await;
        let miss = cache.get("users:1").await;
        assert!(miss.is_none());
    }
}
