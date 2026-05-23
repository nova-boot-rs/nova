use std::fmt;

#[derive(Debug, Clone)]
pub struct ResilienceError {
    message: String,
}

impl ResilienceError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ResilienceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ResilienceError {}

/// Lua script return values supported by resilience stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LuaValue {
    Nil,
    Integer(i64),
    Bytes(Vec<u8>),
    Array(Vec<LuaValue>),
    Status(String),
    Ok,
}

impl LuaValue {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(value) => Some(value.as_slice()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.as_bytes()
            .and_then(|value| std::str::from_utf8(value).ok())
    }
}

/// Minimal distributed key-value operations used by resilience backends.
#[async_trait::async_trait]
pub trait ResilienceStore: Send + Sync + 'static {
    async fn incr(&self, key: &str) -> Result<i64, ResilienceError>;
    async fn get_i64(&self, key: &str) -> Result<Option<i64>, ResilienceError>;
    async fn set_ex(&self, key: &str, val: i64, ttl_seconds: usize) -> Result<(), ResilienceError>;
    async fn del(&self, key: &str) -> Result<(), ResilienceError>;
    async fn eval_lua(
        &self,
        script: &str,
        keys: &[&str],
        args: &[&str],
    ) -> Result<LuaValue, ResilienceError>;
}

/// Redis-backed implementation (optional, behind feature flag `redis-store`).
#[cfg(feature = "redis-store")]
pub mod redis_store {
    use super::{LuaValue, ResilienceError, ResilienceStore};
    use redis::AsyncCommands;
    use redis::Client;

    #[derive(Clone)]
    pub struct RedisStore {
        client: Client,
    }

    impl RedisStore {
        pub fn new(url: &str) -> Result<Self, ResilienceError> {
            let client = Client::open(url).map_err(|e| ResilienceError::new(e.to_string()))?;
            Ok(Self { client })
        }
    }

    impl From<redis::Value> for LuaValue {
        fn from(value: redis::Value) -> Self {
            match value {
                redis::Value::Nil => Self::Nil,
                redis::Value::Int(value) => Self::Integer(value),
                redis::Value::Data(value) => Self::Bytes(value),
                redis::Value::Bulk(values) => {
                    Self::Array(values.into_iter().map(Self::from).collect())
                }
                redis::Value::Status(value) => Self::Status(value),
                redis::Value::Okay => Self::Ok,
            }
        }
    }

    #[async_trait::async_trait]
    impl ResilienceStore for RedisStore {
        async fn incr(&self, key: &str) -> Result<i64, ResilienceError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            let v: i64 = conn
                .incr(key, 1)
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            Ok(v)
        }

        async fn get_i64(&self, key: &str) -> Result<Option<i64>, ResilienceError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            let v: Option<i64> = conn
                .get(key)
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            Ok(v)
        }

        async fn set_ex(
            &self,
            key: &str,
            val: i64,
            ttl_seconds: usize,
        ) -> Result<(), ResilienceError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            let () = conn
                .set_ex(key, val, ttl_seconds)
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            Ok(())
        }

        async fn del(&self, key: &str) -> Result<(), ResilienceError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            let () = conn
                .del(key)
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            Ok(())
        }

        async fn eval_lua(
            &self,
            script: &str,
            keys: &[&str],
            args: &[&str],
        ) -> Result<LuaValue, ResilienceError> {
            let mut conn = self
                .client
                .get_tokio_connection()
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;

            let script = redis::Script::new(script);
            let mut invocation = script.prepare_invoke();
            for key in keys {
                invocation.key(*key);
            }
            for arg in args {
                invocation.arg(*arg);
            }

            let value: redis::Value = invocation
                .invoke_async(&mut conn)
                .await
                .map_err(|e| ResilienceError::new(e.to_string()))?;
            Ok(value.into())
        }
    }
}
