use nova_core::{NovaPlugin, async_trait, axum::Extension, axum::Router};
pub use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Schema, Statement,
};
pub use sea_orm_migration::prelude::*;
use serde_json::Value as JsonValue;
// using redis::Cmd::...query_async instead of AsyncCommands trait
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

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
    replicas: Arc<RwLock<Vec<DatabaseConnection>>>,
}

/// Optional pool configuration passed to `connect_with_options`.
#[derive(Default)]
pub struct PoolOptions {
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub connect_timeout_secs: Option<u64>,
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

/// Redis-backed query cache store.
pub struct RedisQueryCache {
    manager: Arc<tokio::sync::Mutex<redis::aio::Connection>>,
}

impl RedisQueryCache {
    /// Create a new RedisQueryCache from a redis connection URL (e.g. `redis://127.0.0.1/`).
    pub async fn new(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let conn = client.get_async_connection().await?;
        Ok(Self {
            manager: Arc::new(tokio::sync::Mutex::new(conn)),
        })
    }
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

#[async_trait]
impl QueryCacheStore for RedisQueryCache {
    async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.manager.lock().await;
        redis::Cmd::get(key)
            .query_async::<_, Option<String>>(&mut *conn)
            .await
            .unwrap_or_default()
    }

    async fn set(&self, key: &str, value: String, ttl: Duration) {
        let mut conn = self.manager.lock().await;
        let _ = redis::Cmd::set_ex(key, value, ttl.as_secs() as usize)
            .query_async::<_, ()>(&mut *conn)
            .await;
    }

    async fn del(&self, key: &str) {
        let mut conn = self.manager.lock().await;
        let _ = redis::Cmd::del(key).query_async::<_, ()>(&mut *conn).await;
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
            replicas: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Connect using `ConnectOptions`—caller can configure pooling options there.
    pub async fn connect_with_options(options: sea_orm::ConnectOptions, allow_drop: bool) -> Self {
        let db = Database::connect(options)
            .await
            .expect("Failed to connect to the database with options");

        Self {
            db,
            allow_drop,
            sync_tasks: Vec::new(),
            cache_store: None,
            replicas: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a replica by database URL. Returns `DbErr` on connection failure.
    pub async fn add_replica_url(&self, url: &str) -> Result<(), DbErr> {
        let conn = Database::connect(url).await?;
        self.replicas.write().await.push(conn);
        Ok(())
    }

    /// Register replicas from a list of URLs; best-effort: returns the number of
    /// successfully added replicas.
    pub async fn register_replicas_from_urls(&self, urls: &[String]) -> usize {
        let mut added = 0usize;
        for url in urls {
            if self.add_replica_url(url).await.is_ok() {
                added += 1;
            }
        }
        added
    }

    /// Add an existing `DatabaseConnection` as a replica.
    pub async fn add_replica_conn(&self, conn: DatabaseConnection) {
        self.replicas.write().await.push(conn);
    }

    /// Return a snapshot of current replica count.
    pub async fn replica_count(&self) -> usize {
        self.replicas.read().await.len()
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

    /// Run migrations using the provided `MigratorTrait` implementation.
    /// Returns a `Result` with the underlying `DbErr` on failure.
    pub async fn run_migrations<M>(&self) -> Result<(), DbErr>
    where
        M: MigratorTrait,
    {
        M::up(&self.db, None).await
    }

    /// Run migrations with simple retry logic.
    pub async fn run_migrations_with_retry<M>(
        &self,
        attempts: usize,
        delay: Duration,
    ) -> Result<(), DbErr>
    where
        M: MigratorTrait,
    {
        let mut last_err = None;
        for _ in 0..attempts {
            match M::up(&self.db, None).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_err = Some(e);
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(last_err.expect("migration attempts failed but no error captured"))
    }

    /// Construct a `ReadWritePool` for injection into handlers; clones internal references.
    pub fn read_write_pool(&self) -> ReadWritePool {
        ReadWritePool::new(self.db.clone(), self.replicas.clone())
    }
}

/// Simple read/write pool with round-robin replica selection for reads.
#[derive(Clone)]
pub struct ReadWritePool {
    primary: DatabaseConnection,
    replicas: Arc<RwLock<Vec<DatabaseConnection>>>,
    rr: Arc<AtomicUsize>,
}

impl ReadWritePool {
    pub fn new(
        primary: DatabaseConnection,
        replicas: Arc<RwLock<Vec<DatabaseConnection>>>,
    ) -> Self {
        Self {
            primary,
            replicas,
            rr: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Choose a replica connection for read queries. Async because replicas list is protected by an async lock.
    pub async fn read(&self) -> DatabaseConnection {
        let reps = self.replicas.read().await;
        if reps.is_empty() {
            return self.primary.clone();
        }

        let idx = self.rr.fetch_add(1, Ordering::Relaxed);
        reps[idx % reps.len()].clone()
    }

    /// Return the primary connection for writes.
    pub fn write(&self) -> DatabaseConnection {
        self.primary.clone()
    }

    /// Add a replica connection dynamically.
    pub async fn add_replica(&self, conn: DatabaseConnection) {
        self.replicas.write().await.push(conn);
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
        // Inject a ReadWritePool extension for handlers to use read/write splitting.
        let pool = self.read_write_pool();
        router.layer(Extension(pool))
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
