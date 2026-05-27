use crate::{error::NoSqlError, types::NoSqlIndex};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

/// Adapter abstraction for NoSQL primary document stores.
#[async_trait]
pub trait DocumentStore: Send + Sync {
    /// Get a document by collection and id, returning `None` if not found.
    async fn get(&self, collection: &str, id: &str) -> Result<Option<JsonValue>, NoSqlError>;
    /// Upsert a document into the store.
    async fn upsert(&self, collection: &str, id: &str, doc: JsonValue) -> Result<(), NoSqlError>;
    /// Delete a document from the store.
    async fn delete(&self, collection: &str, id: &str) -> Result<(), NoSqlError>;
    /// Index management helpers proxying to the primary adapter.
    async fn create_index(&self, collection: &str, index: NoSqlIndex) -> Result<(), NoSqlError>;
    /// List indexes for a collection.
    async fn list_indexes(&self, collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError>;
}

/// Optional cache adapter for document payloads.
#[async_trait]
pub trait DocumentCacheStore: Send + Sync {
    /// Get a cached value by key, returning `None` if not found or on error.
    async fn get(&self, key: &str) -> Option<String>;
    /// Set a value in the cache with a TTL in seconds.
    async fn set(&self, key: &str, value: String, ttl_secs: usize);
    /// Delete a value from the cache by key.
    async fn del(&self, key: &str);
}
