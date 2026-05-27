use crate::{cache::QueryCacheStore, tenant::TenantResolver};
use sea_orm::{Database, DatabaseConnection, DbErr};
use std::sync::Arc;

/// Main SQL integration surface.
///
/// `NovaSql` wraps a `sea_orm::DatabaseConnection` and provides helper
/// methods to register replicas, configure caching hooks and run schema
/// synchronization tasks. Create an instance via `NovaSql::connect` or
/// `connect_with_options` and register it as a plugin in `NovaApp`.
pub struct NovaSql {
    /// Primary database connection used for writes.
    pub db: DatabaseConnection,

    /// When true, schema sync may drop unused columns when reconciling models.
    pub allow_drop: bool,

    /// Internal sync tasks to be executed during plugin initialization.
    pub(crate) sync_tasks: Vec<Box<dyn crate::migration::SyncTask>>,

    /// Optional query cache store used by higher-level helpers.
    pub(crate) cache_store: Option<Arc<dyn QueryCacheStore>>,

    /// Replica connections used for read scaling.
    pub(crate) replicas: Arc<tokio::sync::RwLock<Vec<DatabaseConnection>>>,

    /// Optional tenant resolver used by per-tenant middleware.
    pub tenant_resolver: Option<Arc<dyn TenantResolver>>,
}

/// Optional pool configuration passed to `connect_with_options`.
#[derive(Default)]
pub struct PoolOptions {
    /// Maximum number of pooled connections.
    pub max_connections: Option<u32>,
    /// Minimum number of pooled connections.
    pub min_connections: Option<u32>,
    /// Connect timeout in seconds.
    pub connect_timeout_secs: Option<u64>,
}

impl NovaSql {
    /// Connect to a database using a connection URL.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # tokio_test::block_on(async {
    /// let sql = nova_sql::NovaSql::connect("sqlite::memory:", false).await;
    /// let pool = sql.read_write_pool();
    /// # });
    /// ```
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
    ///
    /// Use this when you need to fine-tune the underlying connection options
    /// such as timeouts or TLS settings.
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
    ///
    /// This method attempts to open a new connection to `url` and, on
    /// success, appends it to the replica list used by `ReadWritePool` for
    /// read operations.
    pub async fn add_replica_url(&self, url: &str) -> Result<(), DbErr> {
        let conn = Database::connect(url).await?;
        self.replicas.write().await.push(conn);
        Ok(())
    }

    /// Register replicas from a list of URLs; best-effort: returns the number of
    /// successfully added replicas.
    ///
    /// Useful when bootstrapping a pool from configuration where some replica
    /// endpoints may be temporarily unavailable.
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

    /// Configure a query cache store used by higher-level helpers.
    ///
    /// Example: supply a shared `QueryCacheStore` implementation to enable
    /// caching in projection or query helpers.
    pub fn with_cache_store(mut self, cache_store: Arc<dyn QueryCacheStore>) -> Self {
        self.cache_store = Some(cache_store);
        self
    }

    /// Attach a tenant resolver for multi-tenant deployments.
    ///
    /// The resolver is used by tenant middleware to determine the active
    /// tenant for each request. Implement the `TenantResolver` trait and
    /// provide an `Arc`-wrapped instance via this builder method.
    pub fn with_tenant_resolver(mut self, resolver: impl TenantResolver) -> Self {
        self.tenant_resolver = Some(Arc::new(resolver));
        self
    }
}
