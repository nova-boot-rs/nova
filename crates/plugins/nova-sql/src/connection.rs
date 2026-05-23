use crate::{cache::QueryCacheStore, tenant::TenantResolver};
use sea_orm::{Database, DatabaseConnection, DbErr};
use std::sync::Arc;

pub struct NovaSql {
    pub db: DatabaseConnection,
    pub allow_drop: bool,
    pub(crate) sync_tasks: Vec<Box<dyn crate::migration::SyncTask>>,
    pub(crate) cache_store: Option<Arc<dyn QueryCacheStore>>,
    pub(crate) replicas: Arc<tokio::sync::RwLock<Vec<DatabaseConnection>>>,
    pub tenant_resolver: Option<Arc<dyn TenantResolver>>,
}

/// Optional pool configuration passed to `connect_with_options`.
#[derive(Default)]
pub struct PoolOptions {
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub connect_timeout_secs: Option<u64>,
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
            replicas: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            tenant_resolver: None,
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
            replicas: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            tenant_resolver: None,
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

    pub fn with_tenant_resolver(mut self, resolver: impl TenantResolver) -> Self {
        self.tenant_resolver = Some(Arc::new(resolver));
        self
    }
}
