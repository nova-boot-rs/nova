use crate::{
    error::NoSqlError,
    memory::InMemoryDocumentStore,
    mongo::MongoDocumentStore,
    traits::{DocumentCacheStore, DocumentStore},
    types::NoSqlIndex,
    wrapper::NovaNoSql,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
struct TestCache {
    inner: Mutex<HashMap<String, String>>,
}

#[async_trait::async_trait]
impl DocumentCacheStore for TestCache {
    async fn get(&self, key: &str) -> Option<String> {
        self.inner.lock().await.get(key).cloned()
    }

    async fn set(&self, key: &str, value: String, _ttl_secs: usize) {
        self.inner.lock().await.insert(key.to_string(), value);
    }

    async fn del(&self, key: &str) {
        self.inner.lock().await.remove(key);
    }
}

struct FailingStore;

#[async_trait::async_trait]
impl DocumentStore for FailingStore {
    async fn get(&self, _collection: &str, _id: &str) -> Result<Option<JsonValue>, NoSqlError> {
        Err(NoSqlError::Backend(
            "primary should not be called".to_string(),
        ))
    }

    async fn upsert(
        &self,
        _collection: &str,
        _id: &str,
        _doc: JsonValue,
    ) -> Result<(), NoSqlError> {
        Err(NoSqlError::Backend("not used".to_string()))
    }

    async fn delete(&self, _collection: &str, _id: &str) -> Result<(), NoSqlError> {
        Err(NoSqlError::Backend("not used".to_string()))
    }

    async fn create_index(&self, _collection: &str, _index: NoSqlIndex) -> Result<(), NoSqlError> {
        Err(NoSqlError::Backend("not used".to_string()))
    }

    async fn list_indexes(&self, _collection: &str) -> Result<Vec<NoSqlIndex>, NoSqlError> {
        Err(NoSqlError::Backend("not used".to_string()))
    }
}

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

    nosql
        .upsert("users", &user.id, &user)
        .await
        .expect("upsert ok");
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

#[tokio::test]
async fn mongo_invalid_uri_returns_error() {
    let res = MongoDocumentStore::new("not-a-uri", "nova").await;
    assert!(res.is_err());
}

#[tokio::test]
async fn cache_hit_bypasses_primary_store() {
    let cache = Arc::new(TestCache::default());
    let key = "nosql:users:u-1";
    cache
        .set(
            key,
            "{\"id\":\"u-1\",\"email\":\"cached@nova.rs\"}".to_string(),
            30,
        )
        .await;

    let nosql = NovaNoSql::new(Arc::new(FailingStore)).with_cache(cache);
    let loaded: Option<UserDoc> = nosql
        .get("users", "u-1")
        .await
        .expect("cache hit should succeed");

    assert_eq!(loaded.map(|u| u.email), Some("cached@nova.rs".to_string()));
}

#[tokio::test]
async fn delete_invalidates_cache_entry() {
    let cache = Arc::new(TestCache::default());
    let store = Arc::new(InMemoryDocumentStore::default());
    let nosql = NovaNoSql::new(store).with_cache(cache.clone());

    let user = UserDoc {
        id: "u-del".to_string(),
        email: "delete@nova.rs".to_string(),
    };

    nosql
        .upsert("users", &user.id, &user)
        .await
        .expect("upsert should populate cache");

    let cache_key = "nosql:users:u-del";
    assert!(cache.get(cache_key).await.is_some());

    nosql
        .delete("users", &user.id)
        .await
        .expect("delete should remove from store and cache");

    assert!(cache.get(cache_key).await.is_none());
}
