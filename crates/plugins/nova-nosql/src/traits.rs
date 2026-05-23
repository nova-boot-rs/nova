use crate::{error::NoSqlError, types::NoSqlIndex};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

/// Adapter abstraction for NoSQL primary document stores.
#[async_trait]
pub trait DocumentStore: Send + Sync {
    async fn get(&self, collection: &str, id: &str) -> Result<Option<JsonValue>, NoSqlError>;
    async fn upsert(&self, collection: &str, id: &str, doc: JsonValue) -> Result<(), NoSqlError>;
    async fn delete(&self, collection: &str, id: &str) -> Result<(), NoSqlError>;
    async fn create_index(&self, collection: &str, index: NoSqlIndex) -> Result<(), NoSqlError>;
    async fn list_indexes(&self, collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError>;
}

/// Optional cache adapter for document payloads.
#[async_trait]
pub trait DocumentCacheStore: Send + Sync {
    async fn get(&self, key: &str) -> Option<String>;
    async fn set(&self, key: &str, value: String, ttl_secs: usize);
    async fn del(&self, key: &str);
}