use crate::connection::NovaSql;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

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
    pub async fn cached_json<F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        fetcher: F,
    ) -> Result<JsonValue, sea_orm::DbErr>
    where
        F: FnOnce(&sea_orm::DatabaseConnection) -> Fut,
        Fut: std::future::Future<Output = Result<JsonValue, sea_orm::DbErr>>,
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
}