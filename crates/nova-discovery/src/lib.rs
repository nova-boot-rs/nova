use std::fmt;

#[derive(Debug, Clone)]
pub struct DiscoveryError {
    message: String,
}

impl DiscoveryError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DiscoveryError {}

/// Minimal distributed key-value operations used by resilience backends.
#[async_trait::async_trait]
pub trait DistributedStore: Send + Sync + 'static {
    async fn incr(&self, key: &str) -> Result<i64, DiscoveryError>;
    async fn get_i64(&self, key: &str) -> Result<Option<i64>, DiscoveryError>;
    async fn set_ex(&self, key: &str, val: i64, ttl_seconds: usize) -> Result<(), DiscoveryError>;
    async fn del(&self, key: &str) -> Result<(), DiscoveryError>;
}

/// Redis-backed implementation (optional, behind feature flag `redis-store`).
#[cfg(feature = "redis-store")]
pub mod redis_store {
    use super::{DiscoveryError, DistributedStore};
    use redis::AsyncCommands;
    use redis::Client;

    #[derive(Clone)]
    pub struct RedisStore {
        client: Client,
    }

    impl RedisStore {
        pub fn new(url: &str) -> Result<Self, DiscoveryError> {
            let client = Client::open(url).map_err(|e| DiscoveryError::new(e.to_string()))?;
            Ok(Self { client })
        }
    }

    #[async_trait::async_trait]
    impl DistributedStore for RedisStore {
        async fn incr(&self, key: &str) -> Result<i64, DiscoveryError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| DiscoveryError::new(e.to_string()))?;
            let v: i64 = conn
                .incr(key, 1)
                .await
                .map_err(|e| DiscoveryError::new(e.to_string()))?;
            Ok(v)
        }

        async fn get_i64(&self, key: &str) -> Result<Option<i64>, DiscoveryError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| DiscoveryError::new(e.to_string()))?;
            let v: Option<i64> = conn
                .get(key)
                .await
                .map_err(|e| DiscoveryError::new(e.to_string()))?;
            Ok(v)
        }

        async fn set_ex(&self, key: &str, val: i64, ttl_seconds: usize) -> Result<(), DiscoveryError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| DiscoveryError::new(e.to_string()))?;
            let () = conn
                .set_ex(key, val, ttl_seconds)
                .await
                .map_err(|e| DiscoveryError::new(e.to_string()))?;
            Ok(())
        }

        async fn del(&self, key: &str) -> Result<(), DiscoveryError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| DiscoveryError::new(e.to_string()))?;
            let () = conn
                .del(key)
                .await
                .map_err(|e| DiscoveryError::new(e.to_string()))?;
            Ok(())
        }
    }
}
