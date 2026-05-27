use crate::{error::NoSqlError, traits::DocumentStore, types::NoSqlIndex};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory adapter useful for tests and local development.
#[derive(Default)]
pub struct InMemoryDocumentStore {
    data: Arc<RwLock<HashMap<String, HashMap<String, JsonValue>>>>,
    indexes: Arc<RwLock<HashMap<String, Vec<NoSqlIndex>>>>,
}

/// The `InMemoryDocumentStore` implements the `DocumentStore` trait, allowing it to be used as a primary document store in Nova applications. It uses `RwLock` to manage concurrent access to
/// the underlying data and index structures, making it suitable for asynchronous contexts. This adapter is ideal for testing and local development but is not recommended for production use due to its lack of persistence and scalability.
/// The implementation provides basic CRUD operations for documents, as well as index management functions that allow you to create and list indexes for collections. The `get`, `upsert`, and `delete` methods operate on a nested `HashMap` structure where the first level key is the collection name and the second level key is the document ID.
/// The `create_index` and `list_indexes` methods manage a separate `HashMap` that tracks indexes for each collection, allowing you to simulate index creation and retrieval in an in-memory context. Overall, this adapter serves as a simple and effective way to test NoSQL document store functionality without needing to set up an actual database backend.
/// Note that since this is an in-memory store, all data will be lost when the application is restarted, and it does not support advanced features like transactions or complex queries. It is primarily intended for use in unit tests or as a lightweight option during development.
#[async_trait]
impl DocumentStore for InMemoryDocumentStore {
    async fn get(&self, collection: &str, id: &str) -> Result<Option<JsonValue>, NoSqlError> {
        let data = self.data.read().await;
        Ok(data.get(collection).and_then(|c| c.get(id)).cloned())
    }

    async fn upsert(&self, collection: &str, id: &str, doc: JsonValue) -> Result<(), NoSqlError> {
        let mut data = self.data.write().await;
        data.entry(collection.to_string())
            .or_default()
            .insert(id.to_string(), doc);
        Ok(())
    }

    async fn delete(&self, collection: &str, id: &str) -> Result<(), NoSqlError> {
        let mut data = self.data.write().await;
        if let Some(col) = data.get_mut(collection) {
            col.remove(id);
        }
        Ok(())
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
