use crate::{error::NoSqlError, mapper::SerdeDocumentMapper, traits::{DocumentCacheStore, DocumentStore}, types::NoSqlIndex};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct NovaNoSql {
    primary: Arc<dyn DocumentStore>,
    cache: Option<Arc<dyn DocumentCacheStore>>,
}

impl NovaNoSql {
    pub fn new(primary: Arc<dyn DocumentStore>) -> Self {
        Self {
            primary,
            cache: None,
        }
    }

    pub async fn redis_primary(
        url: &str,
        namespace: impl Into<String>,
    ) -> Result<Self, NoSqlError> {
        let store = crate::redis::RedisDocumentStore::new(url, namespace).await?;
        Ok(Self::new(Arc::new(store)))
    }

    pub async fn mongo_primary(uri: &str, database: &str) -> Result<Self, NoSqlError> {
        let store = crate::mongo::MongoDocumentStore::new(uri, database).await?;
        Ok(Self::new(Arc::new(store)))
    }

    pub fn with_cache(mut self, cache: Arc<dyn DocumentCacheStore>) -> Self {
        self.cache = Some(cache);
        self
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<Option<T>, NoSqlError> {
        if let Some(cache) = &self.cache {
            let key = format!("nosql:{collection}:{id}");
            if let Some(raw) = cache.get(&key).await {
                let value: T = serde_json::from_str(&raw)
                    .map_err(|e| NoSqlError::Serialization(e.to_string()))?;
                return Ok(Some(value));
            }
        }

        match self.primary.get(collection, id).await? {
            Some(value) => {
                if let Some(cache) = &self.cache
                    && let Ok(raw) = serde_json::to_string(&value)
                {
                    let key = format!("nosql:{collection}:{id}");
                    cache.set(&key, raw, 30).await;
                }
                SerdeDocumentMapper::from_value(value).map(Some)
            }
            None => Ok(None),
        }
    }

    pub async fn upsert<T: Serialize>(
        &self,
        collection: &str,
        id: &str,
        value: &T,
    ) -> Result<(), NoSqlError> {
        let doc = SerdeDocumentMapper::to_value(value)?;
        self.primary.upsert(collection, id, doc.clone()).await?;

        if let Some(cache) = &self.cache
            && let Ok(raw) = serde_json::to_string(&doc)
        {
            let key = format!("nosql:{collection}:{id}");
            cache.set(&key, raw, 30).await;
        }
        Ok(())
    }

    pub async fn delete(&self, collection: &str, id: &str) -> Result<(), NoSqlError> {
        self.primary.delete(collection, id).await?;
        if let Some(cache) = &self.cache {
            let key = format!("nosql:{collection}:{id}");
            cache.del(&key).await;
        }
        Ok(())
    }

    pub async fn create_index(
        &self,
        collection: &str,
        index: NoSqlIndex,
    ) -> Result<(), NoSqlError> {
        self.primary.create_index(collection, index).await
    }

    pub async fn list_indexes(&self, collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError> {
        self.primary.list_indexes(collection).await
    }
}