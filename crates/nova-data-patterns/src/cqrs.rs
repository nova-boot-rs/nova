use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::info;

use crate::error::CqrsError;

// ---------------------------------------------------------------------------
// Stored event — the canonical persistence representation of a domain event
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: JsonValue,
    pub version: u32,
    pub timestamp_ms: u64,
}

impl StoredEvent {
    pub fn new(
        aggregate_id: impl Into<String>,
        event_type: impl Into<String>,
        payload: JsonValue,
        version: u32,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            aggregate_id: aggregate_id.into(),
            event_type: event_type.into(),
            payload,
            version,
            timestamp_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Command  – mutates state
// Query    – reads state, no side-effects
// ---------------------------------------------------------------------------

#[async_trait]
pub trait Command: Send + Sync {
    type AggregateId: Send;
    async fn execute(self, store: &dyn CommandStore) -> Result<Self::AggregateId, CqrsError>;
}

#[async_trait]
pub trait Query: Send + Sync {
    type Result: Send;
    async fn execute(self, store: &dyn QueryStore) -> Result<Self::Result, CqrsError>;
}

// ---------------------------------------------------------------------------
// Stores – separate read / write contracts
//
// Use concrete serde_json::Value in the signatures so both traits stay
// dyn-compatible (generic methods prevent vtables).
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CommandStore: Send + Sync {
    async fn save_event(&self, event: StoredEvent) -> Result<(), CqrsError>;
    async fn get_events(&self, aggregate_id: &str) -> Result<Vec<StoredEvent>, CqrsError>;
}

#[async_trait]
pub trait QueryStore: Send + Sync {
    async fn get_projection_raw(&self, key: &str) -> Result<Option<JsonValue>, CqrsError>;
    async fn upsert_projection_raw(&self, key: &str, value: JsonValue) -> Result<(), CqrsError>;
}

/// Extension trait with generic convenience methods on any `QueryStore`.
#[async_trait]
pub trait QueryStoreExt: QueryStore {
    async fn get_projection<T: DeserializeOwned + Send>(
        &self,
        key: &str,
    ) -> Result<Option<T>, CqrsError> {
        match self.get_projection_raw(key).await? {
            Some(raw) => Ok(Some(serde_json::from_value(raw)?)),
            None => Ok(None),
        }
    }

    async fn upsert_projection(
        &self,
        key: &str,
        value: &(impl Serialize + Send + Sync),
    ) -> Result<(), CqrsError> {
        let raw = serde_json::to_value(value)?;
        self.upsert_projection_raw(key, raw).await
    }
}

/// Blanket impl so every `QueryStore` also implements `QueryStoreExt`.
impl<T: QueryStore + ?Sized> QueryStoreExt for T {}

// ---------------------------------------------------------------------------
// In-memory store — implements both CommandStore and QueryStore for dev/test
// ---------------------------------------------------------------------------

pub struct InMemoryCqrsStore {
    events: Arc<RwLock<HashMap<String, Vec<StoredEvent>>>>,
    projections: Arc<RwLock<HashMap<String, JsonValue>>>,
}

impl InMemoryCqrsStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            projections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryCqrsStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandStore for InMemoryCqrsStore {
    async fn save_event(&self, event: StoredEvent) -> Result<(), CqrsError> {
        let mut map = self.events.write().await;
        let events = map.entry(event.aggregate_id.clone()).or_default();
        info!(
            aggregate_id = %event.aggregate_id,
            event_type = %event.event_type,
            version = %event.version,
            "in-memory command store: saving event"
        );
        events.push(event);
        Ok(())
    }

    async fn get_events(&self, aggregate_id: &str) -> Result<Vec<StoredEvent>, CqrsError> {
        let map = self.events.read().await;
        Ok(map.get(aggregate_id).cloned().unwrap_or_default())
    }
}

#[async_trait]
impl QueryStore for InMemoryCqrsStore {
    async fn get_projection_raw(&self, key: &str) -> Result<Option<JsonValue>, CqrsError> {
        let map = self.projections.read().await;
        Ok(map.get(key).cloned())
    }

    async fn upsert_projection_raw(&self, key: &str, value: JsonValue) -> Result<(), CqrsError> {
        info!(key = %key, "in-memory query store: upserting projection");
        self.projections
            .write()
            .await
            .insert(key.to_string(), value);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Redis-backed QueryStore for projections
// ---------------------------------------------------------------------------

#[cfg(feature = "redis-store")]
pub struct RedisQueryStore {
    conn: Arc<tokio::sync::Mutex<redis::aio::Connection>>,
    prefix: String,
}

#[cfg(feature = "redis-store")]
impl RedisQueryStore {
    pub async fn new(url: &str) -> Result<Self, CqrsError> {
        let client = redis::Client::open(url).map_err(|e| CqrsError::Backend(e.to_string()))?;
        let conn = client
            .get_async_connection()
            .await
            .map_err(|e| CqrsError::Backend(e.to_string()))?;
        Ok(Self {
            conn: Arc::new(tokio::sync::Mutex::new(conn)),
            prefix: "projection".to_string(),
        })
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    fn key(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, key)
    }
}

#[cfg(feature = "redis-store")]
#[async_trait]
impl QueryStore for RedisQueryStore {
    async fn get_projection_raw(&self, key: &str) -> Result<Option<JsonValue>, CqrsError> {
        let mut conn = self.conn.lock().await;
        let redis_key = self.key(key);
        let raw = redis::Cmd::get(&redis_key)
            .query_async::<_, Option<String>>(&mut *conn)
            .await
            .map_err(|e| CqrsError::Backend(e.to_string()))?;
        match raw {
            Some(json) => {
                let value: JsonValue = serde_json::from_str(&json)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn upsert_projection_raw(&self, key: &str, value: JsonValue) -> Result<(), CqrsError> {
        let mut conn = self.conn.lock().await;
        let redis_key = self.key(key);
        let json = serde_json::to_string(&value)?;
        redis::Cmd::set(&redis_key, json)
            .query_async::<_, ()>(&mut *conn)
            .await
            .map_err(|e| CqrsError::Backend(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct UserProjection {
        id: String,
        email: String,
    }

    #[tokio::test]
    async fn inmemory_cqrs_store_save_and_get_events() {
        let store = InMemoryCqrsStore::new();

        let event = StoredEvent::new(
            "agg-1",
            "UserCreated",
            serde_json::json!({"name":"alice"}),
            1,
        );
        store
            .save_event(event.clone())
            .await
            .expect("save should succeed");

        let events = store
            .get_events("agg-1")
            .await
            .expect("get events should succeed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].aggregate_id, "agg-1");
        assert_eq!(events[0].event_type, "UserCreated");
        assert_eq!(events[0].version, 1);
    }

    #[tokio::test]
    async fn inmemory_cqrs_store_get_events_empty_aggregate() {
        let store = InMemoryCqrsStore::new();
        let events = store
            .get_events("nonexistent")
            .await
            .expect("get events should succeed");
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn inmemory_cqrs_store_upsert_and_read_projection() {
        let store = InMemoryCqrsStore::new();

        let proj = UserProjection {
            id: "u-1".to_string(),
            email: "alice@example.com".to_string(),
        };
        store
            .upsert_projection("user:u-1", &proj)
            .await
            .expect("upsert should succeed");

        let loaded: Option<UserProjection> = store
            .get_projection("user:u-1")
            .await
            .expect("get should succeed");
        assert_eq!(loaded, Some(proj));
    }

    #[tokio::test]
    async fn inmemory_cqrs_store_missing_projection_returns_none() {
        let store = InMemoryCqrsStore::new();
        let loaded: Option<UserProjection> = store
            .get_projection("missing")
            .await
            .expect("get should succeed");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn command_executes_and_stores_event() {
        use super::Command;

        struct CreateUser {
            name: String,
        }

        #[async_trait]
        impl Command for CreateUser {
            type AggregateId = String;

            async fn execute(self, store: &dyn CommandStore) -> Result<String, CqrsError> {
                let event = StoredEvent::new(
                    &self.name,
                    "UserCreated",
                    serde_json::json!({"name": self.name}),
                    1,
                );
                store.save_event(event).await?;
                Ok(self.name)
            }
        }

        let store = InMemoryCqrsStore::new();
        let cmd = CreateUser {
            name: "bob".to_string(),
        };
        let agg_id = cmd.execute(&store).await.expect("command should succeed");
        assert_eq!(agg_id, "bob");

        let events = store.get_events("bob").await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "UserCreated");
    }

    #[tokio::test]
    async fn query_reads_projection() {
        use super::Query;

        struct GetUser {
            key: String,
        }

        #[async_trait]
        impl Query for GetUser {
            type Result = Option<UserProjection>;

            async fn execute(
                self,
                store: &dyn QueryStore,
            ) -> Result<Option<UserProjection>, CqrsError> {
                store.get_projection(&self.key).await
            }
        }

        let store = InMemoryCqrsStore::new();
        let proj = UserProjection {
            id: "u-2".to_string(),
            email: "carol@example.com".to_string(),
        };
        store.upsert_projection("user:u-2", &proj).await.unwrap();

        let query = GetUser {
            key: "user:u-2".to_string(),
        };
        let result: Option<UserProjection> =
            query.execute(&store).await.expect("query should succeed");
        assert_eq!(result, Some(proj));
    }
}
