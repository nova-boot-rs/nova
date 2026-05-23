use crate::{error::NoSqlError, traits::{DocumentCacheStore, DocumentStore}, types::NoSqlIndex};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Redis adapter that can act as both primary document store and cache backend.
pub struct RedisDocumentStore {
    namespace: String,
    conn: Arc<Mutex<redis::aio::Connection>>,
    indexes: Arc<RwLock<HashMap<String, Vec<NoSqlIndex>>>>,
}

impl RedisDocumentStore {
    pub async fn new(url: &str, namespace: impl Into<String>) -> Result<Self, NoSqlError> {
        let client = redis::Client::open(url).map_err(|e| NoSqlError::Backend(e.to_string()))?;
        let conn = client
            .get_async_connection()
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?;
        Ok(Self {
            namespace: namespace.into(),
            conn: Arc::new(Mutex::new(conn)),
            indexes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn key(&self, collection: &str, id: &str) -> String {
        format!("{}:{collection}:{id}", self.namespace)
    }
}

#[async_trait]
impl DocumentStore for RedisDocumentStore {
    async fn get(&self, collection: &str, id: &str) -> Result<Option<JsonValue>, NoSqlError> {
        let mut conn = self.conn.lock().await;
        let key = self.key(collection, id);
        let raw = redis::Cmd::get(&key)
            .query_async::<_, Option<String>>(&mut *conn)
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))?;

        match raw {
            Some(json) => serde_json::from_str::<JsonValue>(&json)
                .map(Some)
                .map_err(|e| NoSqlError::Serialization(e.to_string())),
            None => Ok(None),
        }
    }

    async fn upsert(&self, collection: &str, id: &str, doc: JsonValue) -> Result<(), NoSqlError> {
        let mut conn = self.conn.lock().await;
        let key = self.key(collection, id);
        let json =
            serde_json::to_string(&doc).map_err(|e| NoSqlError::Serialization(e.to_string()))?;

        redis::Cmd::set(&key, json)
            .query_async::<_, ()>(&mut *conn)
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<(), NoSqlError> {
        let mut conn = self.conn.lock().await;
        let key = self.key(collection, id);
        redis::Cmd::del(&key)
            .query_async::<_, ()>(&mut *conn)
            .await
            .map_err(|e| NoSqlError::Backend(e.to_string()))
    }

    async fn create_index(&self, collection: &str, index: NoSqlIndex) -> Result<(), NoSqlError> {
        let mut indexes = self.indexes.write().await;
        indexes
            .entry(collection.to_string())
            .or_default()
            .push(index);
        Ok(())
    }

    async fn list_indexes(&self, collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError> {
        let indexes = self.indexes.read().await;
        Ok(indexes.get(collection).cloned().unwrap_or_default())
    }
}

#[async_trait]
impl DocumentCacheStore for RedisDocumentStore {
    async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.conn.lock().await;
        redis::Cmd::get(key)
            .query_async::<_, Option<String>>(&mut *conn)
            .await
            .unwrap_or_default()
    }

    async fn set(&self, key: &str, value: String, ttl_secs: usize) {
        let mut conn = self.conn.lock().await;
        let _ = redis::Cmd::set_ex(key, value, ttl_secs)
            .query_async::<_, ()>(&mut *conn)
            .await;
    }

    async fn del(&self, key: &str) {
        let mut conn = self.conn.lock().await;
        let _ = redis::Cmd::del(key).query_async::<_, ()>(&mut *conn).await;
    }
}