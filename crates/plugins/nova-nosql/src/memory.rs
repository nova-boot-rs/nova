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
