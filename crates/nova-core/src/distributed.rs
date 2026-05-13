use crate::NovaError;
use async_trait::async_trait;

/// Minimal distributed key-value operations used by resilience backends.
#[async_trait]
pub trait DistributedStore: Send + Sync + 'static {
    async fn incr(&self, key: &str) -> Result<i64, NovaError>;
    async fn get_i64(&self, key: &str) -> Result<Option<i64>, NovaError>;
    async fn set_ex(&self, key: &str, val: i64, ttl_seconds: usize) -> Result<(), NovaError>;
    async fn del(&self, key: &str) -> Result<(), NovaError>;
}

/// Redis-backed implementation (optional, behind feature flag `redis-store`).
#[cfg(feature = "redis-store")]
pub mod redis_store {
    use super::DistributedStore;
    use crate::NovaError;
    use redis::AsyncCommands;
    use redis::Client;
    use redis::aio::ConnectionManager;

    #[derive(Clone)]
    pub struct RedisStore {
        mgr: ConnectionManager,
    }

    impl RedisStore {
        pub async fn new(url: &str) -> Result<Self, NovaError> {
            let client = Client::open(url).map_err(|e| NovaError::InternalError(e.to_string()))?;
            let mgr = client
                .get_tokio_connection_manager()
                .await
                .map_err(|e| NovaError::InternalError(e.to_string()))?;
            Ok(Self { mgr })
        }
    }

    #[async_trait]
    impl DistributedStore for RedisStore {
        async fn incr(&self, key: &str) -> Result<i64, NovaError> {
            let mut conn = self.mgr.clone();
            let v: i64 = conn
                .incr(key, 1)
                .await
                .map_err(|e| NovaError::InternalError(e.to_string()))?;
            Ok(v)
        }

        async fn get_i64(&self, key: &str) -> Result<Option<i64>, NovaError> {
            let mut conn = self.mgr.clone();
            let v: Option<i64> = conn
                .get(key)
                .await
                .map_err(|e| NovaError::InternalError(e.to_string()))?;
            Ok(v)
        }

        async fn set_ex(&self, key: &str, val: i64, ttl_seconds: usize) -> Result<(), NovaError> {
            let mut conn = self.mgr.clone();
            let () = conn
                .set_ex(key, val, ttl_seconds)
                .await
                .map_err(|e| NovaError::InternalError(e.to_string()))?;
            Ok(())
        }

        async fn del(&self, key: &str) -> Result<(), NovaError> {
            let mut conn = self.mgr.clone();
            let () = conn
                .del(key)
                .await
                .map_err(|e| NovaError::InternalError(e.to_string()))?;
            Ok(())
        }
    }
}
