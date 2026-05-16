use async_trait::async_trait;
use nova_core::{NovaPlugin, async_trait as nova_async_trait, axum::Extension, axum::Router};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Describes an index for a document collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoSqlIndex {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
}

impl NoSqlIndex {
    pub fn new(name: impl Into<String>, fields: Vec<String>, unique: bool) -> Self {
        Self {
            name: name.into(),
            fields,
            unique,
        }
    }
}

#[derive(Debug)]
pub enum NoSqlError {
    Backend(String),
    Serialization(String),
    NotImplemented(&'static str),
}

impl fmt::Display for NoSqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "backend error: {msg}"),
            Self::Serialization(msg) => write!(f, "serialization error: {msg}"),
            Self::NotImplemented(msg) => write!(f, "not implemented: {msg}"),
        }
    }
}

impl std::error::Error for NoSqlError {}

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

/// Generic serde mapping helpers for NoSQL documents.
pub struct SerdeDocumentMapper;

impl SerdeDocumentMapper {
    pub fn to_value<T: Serialize>(value: &T) -> Result<JsonValue, NoSqlError> {
        serde_json::to_value(value).map_err(|e| NoSqlError::Serialization(e.to_string()))
    }

    pub fn from_value<T: DeserializeOwned>(value: JsonValue) -> Result<T, NoSqlError> {
        serde_json::from_value(value).map_err(|e| NoSqlError::Serialization(e.to_string()))
    }
}

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
        indexes.entry(collection.to_string()).or_default().push(index);
        Ok(())
    }

    async fn list_indexes(&self, collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError> {
        let indexes = self.indexes.read().await;
        Ok(indexes.get(collection).cloned().unwrap_or_default())
    }
}

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
        let json = serde_json::to_string(&doc).map_err(|e| NoSqlError::Serialization(e.to_string()))?;

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
        indexes.entry(collection.to_string()).or_default().push(index);
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

/// Mongo adapter scaffold. This is intentionally lightweight until a full client is wired.
pub struct MongoDocumentStore {
    pub uri: String,
    pub database: String,
}

impl MongoDocumentStore {
    pub fn new(uri: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            database: database.into(),
        }
    }
}

#[async_trait]
impl DocumentStore for MongoDocumentStore {
    async fn get(&self, _collection: &str, _id: &str) -> Result<Option<JsonValue>, NoSqlError> {
        Err(NoSqlError::NotImplemented("MongoDB runtime client wiring is pending"))
    }

    async fn upsert(&self, _collection: &str, _id: &str, _doc: JsonValue) -> Result<(), NoSqlError> {
        Err(NoSqlError::NotImplemented("MongoDB runtime client wiring is pending"))
    }

    async fn delete(&self, _collection: &str, _id: &str) -> Result<(), NoSqlError> {
        Err(NoSqlError::NotImplemented("MongoDB runtime client wiring is pending"))
    }

    async fn create_index(&self, _collection: &str, _index: NoSqlIndex) -> Result<(), NoSqlError> {
        Err(NoSqlError::NotImplemented("MongoDB runtime client wiring is pending"))
    }

    async fn list_indexes(&self, _collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError> {
        Err(NoSqlError::NotImplemented("MongoDB runtime client wiring is pending"))
    }
}

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

    pub async fn redis_primary(url: &str, namespace: impl Into<String>) -> Result<Self, NoSqlError> {
        let store = RedisDocumentStore::new(url, namespace).await?;
        Ok(Self::new(Arc::new(store)))
    }

    pub fn with_cache(mut self, cache: Arc<dyn DocumentCacheStore>) -> Self {
        self.cache = Some(cache);
        self
    }

    pub async fn get<T: DeserializeOwned>(&self, collection: &str, id: &str) -> Result<Option<T>, NoSqlError> {
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

    pub async fn upsert<T: Serialize>(&self, collection: &str, id: &str, value: &T) -> Result<(), NoSqlError> {
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

    pub async fn create_index(&self, collection: &str, index: NoSqlIndex) -> Result<(), NoSqlError> {
        self.primary.create_index(collection, index).await
    }

    pub async fn list_indexes(&self, collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError> {
        self.primary.list_indexes(collection).await
    }
}

#[nova_async_trait]
impl NovaPlugin for NovaNoSql {
    fn name(&self) -> &'static str {
        "NovaNoSql"
    }

    async fn on_init(&self) {
        println!("🧩 Initializing NoSQL Plugin...");
    }

    fn extend_router(&self, router: Router<()>) -> Router<()> {
        router.layer(Extension(self.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct UserDoc {
        id: String,
        email: String,
    }

    #[tokio::test]
    async fn document_mapping_round_trip() {
        let store = Arc::new(InMemoryDocumentStore::default());
        let nosql = NovaNoSql::new(store);

        let user = UserDoc {
            id: "u-1".to_string(),
            email: "a@nova.rs".to_string(),
        };

        nosql.upsert("users", &user.id, &user).await.expect("upsert ok");
        let loaded: Option<UserDoc> = nosql.get("users", "u-1").await.expect("get ok");
        assert_eq!(loaded, Some(user));
    }

    #[tokio::test]
    async fn index_management_is_recorded() {
        let store = Arc::new(InMemoryDocumentStore::default());
        let nosql = NovaNoSql::new(store);

        nosql
            .create_index(
                "users",
                NoSqlIndex::new("users_email_idx", vec!["email".to_string()], true),
            )
            .await
            .expect("create index");

        let indexes = nosql.list_indexes("users").await.expect("list indexes");
        assert_eq!(indexes.len(), 1);
        assert_eq!(indexes[0].name, "users_email_idx");
        assert_eq!(indexes[0].fields, vec!["email".to_string()]);
        assert!(indexes[0].unique);
    }
}
