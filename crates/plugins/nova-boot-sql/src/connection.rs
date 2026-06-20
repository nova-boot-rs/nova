use crate::{cache::QueryCacheStore, tenant::TenantResolver};
use sea_orm::{Database, DatabaseConnection, DbErr};
use std::sync::Arc;

/// Main SQL integration surface for Nova applications.
///
/// `NovaSql` wraps a [`sea_orm::DatabaseConnection`] and provides a builder
/// API for configuring replica connections, query caching, schema
/// synchronisation, and multi-tenant resolution.
///
/// Once configured, call [`read_write_pool`](Self::read_write_pool) to obtain a
/// [`ReadWritePool`](crate::ReadWritePool) that can be injected into Axum
/// handlers via [`NovaDb`](crate::NovaDb).
///
/// # Basic usage
///
/// ```rust,ignore
/// use nova_boot_sql::NovaSql;
///
/// let sql = NovaSql::connect("postgres://localhost/mydb", false).await;
/// let pool = sql.read_write_pool();
/// ```
pub struct NovaSql {
    /// Primary database connection used for writes.
    pub db: DatabaseConnection,

    /// When `true`, schema synchronisation is allowed to drop columns that
    /// exist in the database but are not present in the entity definitions.
    ///
    /// Default: `false`.
    pub allow_drop: bool,

    /// Migration / schema-sync tasks queued during construction.
    pub(crate) sync_tasks: Vec<Box<dyn crate::migration::SyncTask>>,

    /// Optional query-cache store wired in by [`with_cache_store`](Self::with_cache_store).
    pub(crate) cache_store: Option<Arc<dyn QueryCacheStore>>,

    /// Replica database connections used for read scaling.
    pub(crate) replicas: Arc<std::sync::RwLock<Vec<DatabaseConnection>>>,

    /// Optional tenant resolver injected by [`with_tenant_resolver`](Self::with_tenant_resolver).
    pub tenant_resolver: Option<Arc<dyn TenantResolver>>,
}

/// Optional pool-level configuration for [`NovaSql::connect_with_options`].
#[derive(Default)]
pub struct PoolOptions {
    /// Maximum number of connections in the pool (delegated to the underlying
    /// SQL driver).
    pub max_connections: Option<u32>,
    /// Minimum number of idle connections to maintain.
    pub min_connections: Option<u32>,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: Option<u64>,
}

impl NovaSql {
    /// Open a connection to a database identified by a URL.
    ///
    /// The URL format is driver-specific (e.g. `sqlite::memory:`, `postgres://…`,
    /// `mysql://…`). This is a convenience wrapper around [`sea_orm::Database::connect`].
    ///
    /// # Panics
    ///
    /// Panics on connection failure. Use [`connect_with_options`](Self::connect_with_options)
    /// for finer-grained error handling.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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
            replicas: Arc::new(std::sync::RwLock::new(Vec::new())),
            tenant_resolver: None,
        }
    }

    /// Open a connection with full [`ConnectOptions`](sea_orm::ConnectOptions).
    ///
    /// Use this when you need to configure pooling limits, timeouts, or TLS
    /// settings that cannot be expressed through a plain URL string.
    pub async fn connect_with_options(options: sea_orm::ConnectOptions, allow_drop: bool) -> Self {
        let db = Database::connect(options)
            .await
            .expect("Failed to connect to the database with options");

        Self {
            db,
            allow_drop,
            sync_tasks: Vec::new(),
            cache_store: None,
            replicas: Arc::new(std::sync::RwLock::new(Vec::new())),
            tenant_resolver: None,
        }
    }

    /// Open a new connection to `url` and register it as a read replica.
    ///
    /// On success the new connection is appended to the replica list that
    /// [`ReadWritePool`](crate::ReadWritePool) uses for round-robin read
    /// distribution.
    ///
    /// # Errors
    ///
    /// Returns [`DbErr`] when the database at `url` cannot be reached.
    pub async fn add_replica_url(&self, url: &str) -> Result<(), DbErr> {
        let conn = Database::connect(url).await?;
        self.replicas.write().expect("lock poisoned").push(conn);
        Ok(())
    }

    /// Best-effort registration of multiple replica URLs.
    ///
    /// URLs that fail to connect are silently skipped. Returns the number of
    /// replicas that were successfully added.
    ///
    /// Useful when bootstrapping from a configuration file where some endpoints
    /// may be temporarily unavailable.
    pub async fn register_replicas_from_urls(&self, urls: &[String]) -> usize {
        let mut added = 0usize;
        for url in urls {
            if self.add_replica_url(url).await.is_ok() {
                added += 1;
            }
        }
        added
    }

    /// Register an already-connected [`DatabaseConnection`] as a read replica.
    pub async fn add_replica_conn(&self, conn: DatabaseConnection) {
        self.replicas.write().expect("lock poisoned").push(conn);
    }

    /// Return the number of registered read replicas.
    pub async fn replica_count(&self) -> usize {
        self.replicas.read().expect("lock poisoned").len()
    }

    /// Attach a [`QueryCacheStore`] for automatic query-result caching.
    ///
    /// The cache is consulted by higher-level helpers before falling through to
    /// the database. Implementations exist for in-memory caches and Redis-backed
    /// stores (behind the `redis-store` feature).
    pub fn with_cache_store(mut self, cache_store: Arc<dyn QueryCacheStore>) -> Self {
        self.cache_store = Some(cache_store);
        self
    }

    /// Attach a [`TenantResolver`] for multi-tenant request routing.
    ///
    /// The resolver is invoked by the tenant middleware to extract the active
    /// tenant identity from each incoming request.
    pub fn with_tenant_resolver(mut self, resolver: impl TenantResolver) -> Self {
        self.tenant_resolver = Some(Arc::new(resolver));
        self
    }
}
