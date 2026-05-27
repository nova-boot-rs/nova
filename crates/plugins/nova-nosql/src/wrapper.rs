use crate::{
    error::NoSqlError,
    mapper::SerdeDocumentMapper,
    traits::{DocumentCacheStore, DocumentStore},
    types::NoSqlIndex,
};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

/// High-level wrapper around a primary `DocumentStore` and optional cache.
///
/// `NovaNoSql` provides a convenient typed API for handlers to `get`, `upsert`,
/// `delete`, and manage indexes. It composes a primary store adapter (Mongo,
/// Redis, in-memory) and an optional `DocumentCacheStore` for fast reads.
#[derive(Clone)]
pub struct NovaNoSql {
    primary: Arc<dyn DocumentStore>,
    cache: Option<Arc<dyn DocumentCacheStore>>,
}

impl NovaNoSql {
    /// Construct a `NovaNoSql` with a primary adapter.
    pub fn new(primary: Arc<dyn DocumentStore>) -> Self {
        Self {
            primary,
            cache: None,
        }
    }

    /// Convenience constructor for a Redis-backed primary.
    pub async fn redis_primary(
        url: &str,
        namespace: impl Into<String>,
    ) -> Result<Self, NoSqlError> {
        let store = crate::redis::RedisDocumentStore::new(url, namespace).await?;
        Ok(Self::new(Arc::new(store)))
    }

    /// Convenience constructor for a Mongo-backed primary.
    pub async fn mongo_primary(uri: &str, database: &str) -> Result<Self, NoSqlError> {
        let store = crate::mongo::MongoDocumentStore::new(uri, database).await?;
        Ok(Self::new(Arc::new(store)))
    }

    /// Attach an optional cache adapter.
    pub fn with_cache(mut self, cache: Arc<dyn DocumentCacheStore>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Typed `get` that attempts the cache first and falls back to the primary.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # tokio_test::block_on(async {
    /// use nova_nosql::NovaNoSql;
    /// let nosql = NovaNoSql::redis_primary("redis://127.0.0.1:6379", "app").await.unwrap();
    /// let value: Option<MyType> = nosql.get("collection", "id").await.unwrap();
    /// # });
    /// ```
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

    /// Upsert a value into the primary and update the cache when present.
    ///
    /// This performs a JSON-serialization of `value` and stores it in the
    /// primary adapter. When a cache adapter is present the cached entry is
    /// updated as well.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # tokio_test::block_on(async {
    /// use nova_nosql::NovaNoSql;
    /// let nosql = NovaNoSql::redis_primary("redis://127.0.0.1:6379", "app").await.unwrap();
    /// nosql.upsert("users", "u1", &my_value).await.unwrap();
    /// # });
    /// ```
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

    /// Delete a document from primary and cache.
    ///
    /// Removes the document from the primary store and invalidates the cache
    /// key if a cache adapter is configured.
    pub async fn delete(&self, collection: &str, id: &str) -> Result<(), NoSqlError> {
        self.primary.delete(collection, id).await?;
        if let Some(cache) = &self.cache {
            let key = format!("nosql:{collection}:{id}");
            cache.del(&key).await;
        }
        Ok(())
    }

    /// Index management helpers proxying to the primary adapter.
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
