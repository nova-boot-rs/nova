//! Optional bridge adapters connecting CQRS/Saga patterns to storage and
//! messaging systems (sql bridge, nosql projections, messaging saga bus).
//!
//! These adapters are feature-gated; they are compiled only when the
//! corresponding feature (e.g., `sql-bridge`, `nosql-bridge`, `messaging-bridge`)
//! is enabled.
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

use crate::cqrs::{CommandStore, QueryStore, StoredEvent};
use crate::error::CqrsError;

// ---------------------------------------------------------------------------
// SqlCqrsBridge – wraps nova-boot-sql ReadWritePool for CommandStore + QueryStore
// ---------------------------------------------------------------------------

#[cfg(feature = "sql-bridge")]
pub struct SqlCqrsBridge {
    pool: nova_boot_sql::ReadWritePool,
    initialized: AtomicBool,
}

#[cfg(feature = "sql-bridge")]
impl SqlCqrsBridge {
    pub fn new(pool: nova_boot_sql::ReadWritePool) -> Self {
        Self {
            pool,
            initialized: AtomicBool::new(false),
        }
    }

    /// Create the `nova_events` and `nova_projections` tables if they don't
    /// exist.  Idempotent — safe to call repeatedly.
    pub async fn init(&self) -> Result<(), CqrsError> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        let db = self.pool.write();
        let backend = db.get_database_backend();

        let create_events = match backend {
            nova_boot_sql::DbBackend::Sqlite => {
                "\
                CREATE TABLE IF NOT EXISTS nova_events (\
                    id INTEGER PRIMARY KEY AUTOINCREMENT,\
                    aggregate_id TEXT NOT NULL,\
                    event_type  TEXT NOT NULL,\
                    payload     TEXT NOT NULL,\
                    version     INTEGER NOT NULL,\
                    timestamp_ms INTEGER NOT NULL\
                )\
                "
            }
            nova_boot_sql::DbBackend::Postgres => {
                "\
                CREATE TABLE IF NOT EXISTS nova_events (\
                    id SERIAL PRIMARY KEY,\
                    aggregate_id VARCHAR(255) NOT NULL,\
                    event_type  VARCHAR(255) NOT NULL,\
                    payload     JSONB NOT NULL,\
                    version     INTEGER NOT NULL,\
                    timestamp_ms BIGINT NOT NULL\
                )\
                "
            }
            _ => {
                "\
                CREATE TABLE IF NOT EXISTS nova_events (\
                    id BIGINT AUTO_INCREMENT PRIMARY KEY,\
                    aggregate_id VARCHAR(255) NOT NULL,\
                    event_type  VARCHAR(255) NOT NULL,\
                    payload     JSON NOT NULL,\
                    version     INTEGER NOT NULL,\
                    timestamp_ms BIGINT NOT NULL\
                )\
                "
            }
        };

        let create_projections = match backend {
            nova_boot_sql::DbBackend::Sqlite => {
                "\
                CREATE TABLE IF NOT EXISTS nova_projections (\
                    pkey TEXT PRIMARY KEY,\
                    value TEXT NOT NULL\
                )\
                "
            }
            nova_boot_sql::DbBackend::Postgres => {
                "\
                CREATE TABLE IF NOT EXISTS nova_projections (\
                    pkey VARCHAR(255) PRIMARY KEY,\
                    value JSONB NOT NULL\
                )\
                "
            }
            _ => {
                "\
                CREATE TABLE IF NOT EXISTS nova_projections (\
                    pkey VARCHAR(255) PRIMARY KEY,\
                    value JSON NOT NULL\
                )\
                "
            }
        };

        use nova_boot_sql::ConnectionTrait;
        db.execute(nova_boot_sql::Statement::from_string(
            backend,
            create_events,
        ))
        .await
        .map_err(|e| CqrsError::Backend(e.to_string()))?;
        db.execute(nova_boot_sql::Statement::from_string(
            backend,
            create_projections,
        ))
        .await
        .map_err(|e| CqrsError::Backend(e.to_string()))?;

        self.initialized.store(true, Ordering::Release);
        println!("SqlCqrsBridge: tables initialized");
        Ok(())
    }

    async fn ensure_init(&self) -> Result<(), CqrsError> {
        if !self.initialized.load(Ordering::Acquire) {
            self.init().await?;
        }
        Ok(())
    }
}

#[cfg(feature = "sql-bridge")]
#[async_trait]
impl CommandStore for SqlCqrsBridge {
    async fn save_event(&self, event: StoredEvent) -> Result<(), CqrsError> {
        self.ensure_init().await?;
        let db = self.pool.write();
        let backend = db.get_database_backend();
        let payload = serde_json::to_string(&event.payload)?;

        let sql = match backend {
            nova_boot_sql::DbBackend::Sqlite => {
                "INSERT INTO nova_events (aggregate_id, event_type, payload, version, timestamp_ms) VALUES (?1, ?2, ?3, ?4, ?5)"
            }
            _ => {
                "INSERT INTO nova_events (aggregate_id, event_type, payload, version, timestamp_ms) VALUES ($1, $2, $3, $4, $5)"
            }
        };

        let agg_id = event.aggregate_id.clone();
        let evt_type = event.event_type.clone();
        let ver = event.version;

        use nova_boot_sql::ConnectionTrait;
        db.execute(nova_boot_sql::Statement::from_sql_and_values(
            backend,
            sql,
            vec![
                event.aggregate_id.into(),
                event.event_type.into(),
                payload.into(),
                (event.version as i64).into(),
                (event.timestamp_ms as i64).into(),
            ],
        ))
        .await
        .map_err(|e| CqrsError::Backend(e.to_string()))?;

        info!(
            aggregate_id = %agg_id,
            event_type = %evt_type,
            version = %ver,
            "SqlCqrsBridge: event saved"
        );
        Ok(())
    }

    async fn get_events(&self, aggregate_id: &str) -> Result<Vec<StoredEvent>, CqrsError> {
        self.ensure_init().await?;
        let db = self.pool.read().await;
        let backend = db.get_database_backend();

        let sql = match backend {
            nova_boot_sql::DbBackend::Sqlite => {
                "SELECT aggregate_id, event_type, payload, version, timestamp_ms FROM nova_events WHERE aggregate_id = ?1 ORDER BY version ASC"
            }
            _ => {
                "SELECT aggregate_id, event_type, payload, version, timestamp_ms FROM nova_events WHERE aggregate_id = $1 ORDER BY version ASC"
            }
        };

        use nova_boot_sql::ConnectionTrait;
        let rows = db
            .query_all(nova_boot_sql::Statement::from_sql_and_values(
                backend,
                sql,
                vec![aggregate_id.into()],
            ))
            .await
            .map_err(|e| CqrsError::Backend(e.to_string()))?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let payload_str: String = row
                .try_get("", "payload")
                .map_err(|e| CqrsError::Backend(format!("failed to read payload column: {e}")))?;
            let payload: serde_json::Value = serde_json::from_str(&payload_str)
                .map_err(|e| CqrsError::Serialization(e.to_string()))?;

            events.push(StoredEvent {
                aggregate_id: row
                    .try_get("", "aggregate_id")
                    .map_err(|e| CqrsError::Backend(format!("failed to read aggregate_id: {e}")))?,
                event_type: row
                    .try_get("", "event_type")
                    .map_err(|e| CqrsError::Backend(format!("failed to read event_type: {e}")))?,
                payload,
                version: {
                    let v: i64 = row
                        .try_get("", "version")
                        .map_err(|e| CqrsError::Backend(format!("failed to read version: {e}")))?;
                    v as u32
                },
                timestamp_ms: {
                    let t: i64 = row.try_get("", "timestamp_ms").map_err(|e| {
                        CqrsError::Backend(format!("failed to read timestamp_ms: {e}"))
                    })?;
                    t as u64
                },
            });
        }

        Ok(events)
    }
}

#[cfg(feature = "sql-bridge")]
#[async_trait]
impl QueryStore for SqlCqrsBridge {
    async fn get_projection_raw(&self, key: &str) -> Result<Option<JsonValue>, CqrsError> {
        self.ensure_init().await?;
        let db = self.pool.read().await;
        let backend = db.get_database_backend();

        let sql = match backend {
            nova_boot_sql::DbBackend::Sqlite => {
                "SELECT value FROM nova_projections WHERE pkey = ?1"
            }
            _ => "SELECT value FROM nova_projections WHERE pkey = $1",
        };

        use nova_boot_sql::ConnectionTrait;
        let rows = db
            .query_all(nova_boot_sql::Statement::from_sql_and_values(
                backend,
                sql,
                vec![key.into()],
            ))
            .await
            .map_err(|e| CqrsError::Backend(e.to_string()))?;

        if rows.is_empty() {
            return Ok(None);
        }

        let raw: String = rows[0]
            .try_get("", "value")
            .map_err(|e| CqrsError::Backend(format!("failed to read projection value: {e}")))?;
        let value: JsonValue = serde_json::from_str(&raw)?;
        Ok(Some(value))
    }

    async fn upsert_projection_raw(&self, key: &str, value: JsonValue) -> Result<(), CqrsError> {
        self.ensure_init().await?;
        let db = self.pool.write();
        let backend = db.get_database_backend();
        let json = serde_json::to_string(&value)?;

        let sql = match backend {
            nova_boot_sql::DbBackend::Sqlite => {
                "INSERT OR REPLACE INTO nova_projections (pkey, value) VALUES (?1, ?2)"
            }
            nova_boot_sql::DbBackend::Postgres => {
                "INSERT INTO nova_projections (pkey, value) VALUES ($1, $2::jsonb) ON CONFLICT (pkey) DO UPDATE SET value = EXCLUDED.value"
            }
            _ => "REPLACE INTO nova_projections (pkey, value) VALUES (?, ?)",
        };

        use nova_boot_sql::ConnectionTrait;
        db.execute(nova_boot_sql::Statement::from_sql_and_values(
            backend,
            sql,
            vec![key.into(), json.into()],
        ))
        .await
        .map_err(|e| CqrsError::Backend(e.to_string()))?;

        info!(key = %key, "SqlCqrsBridge: projection upserted");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// NoSqlQueryStore – wraps NovaNoSql for projection queries
// ---------------------------------------------------------------------------

#[cfg(feature = "nosql-bridge")]
pub struct NoSqlQueryStore {
    nosql: nova_boot_nosql::NovaNoSql,
    collection: String,
}

#[cfg(feature = "nosql-bridge")]
impl NoSqlQueryStore {
    pub fn new(nosql: nova_boot_nosql::NovaNoSql) -> Self {
        Self {
            nosql,
            collection: "projections".to_string(),
        }
    }

    pub fn with_collection(mut self, name: impl Into<String>) -> Self {
        self.collection = name.into();
        self
    }
}

#[cfg(feature = "nosql-bridge")]
#[async_trait]
impl QueryStore for NoSqlQueryStore {
    async fn get_projection_raw(&self, key: &str) -> Result<Option<JsonValue>, CqrsError> {
        self.nosql
            .get::<JsonValue>(&self.collection, key)
            .await
            .map_err(|e| CqrsError::Backend(e.to_string()))
    }

    async fn upsert_projection_raw(&self, key: &str, value: JsonValue) -> Result<(), CqrsError> {
        self.nosql
            .upsert(&self.collection, key, &value)
            .await
            .map_err(|e| CqrsError::Backend(e.to_string()))?;
        info!(key = %key, collection = %self.collection, "NoSqlQueryStore: projection upserted");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MessagingSagaBus – publishes saga lifecycle events via NovaMessaging
// ---------------------------------------------------------------------------

#[cfg(feature = "messaging-bridge")]
pub struct MessagingSagaBus {
    messaging: nova_boot_messaging::NovaMessaging,
}

#[cfg(feature = "messaging-bridge")]
use serde::Serialize;

#[cfg(feature = "messaging-bridge")]
impl MessagingSagaBus {
    pub fn new(messaging: nova_boot_messaging::NovaMessaging) -> Self {
        Self { messaging }
    }

    /// Publish an event to `saga.{saga_id}.{step}.{status}`.
    pub async fn publish(
        &self,
        saga_id: &str,
        step: &str,
        status: &str,
        payload: &(impl Serialize + Sync),
    ) -> Result<(), crate::error::SagaError> {
        let topic = format!("saga.{saga_id}.{step}.{status}");
        self.messaging
            .publish_json(
                format!("{saga_id}-{step}-{status}"),
                topic,
                "saga.event",
                payload,
            )
            .await
            .map_err(|e| crate::error::SagaError::Aborted(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "sql-bridge"))]
mod sql_bridge_tests {
    use super::*;
    use crate::cqrs::QueryStoreExt;

    #[tokio::test]
    async fn sqlite_round_trip_events_and_projections() {
        let db = nova_boot_sql::Database::connect("sqlite::memory:")
            .await
            .expect("sqlite in-memory connection");

        use std::sync::{Arc, RwLock};
        let pool = nova_boot_sql::ReadWritePool::new(db, Arc::new(RwLock::new(Vec::new())));
        let bridge = SqlCqrsBridge::new(pool);
        bridge.init().await.expect("bridge init should succeed");

        // Save events
        let event1 = StoredEvent::new(
            "order-1",
            "OrderPlaced",
            serde_json::json!({"total": 99.99}),
            1,
        );
        let event2 = StoredEvent::new(
            "order-1",
            "OrderShipped",
            serde_json::json!({"tracking": "TRK1"}),
            2,
        );
        bridge.save_event(event1).await.expect("save event1");
        bridge.save_event(event2).await.expect("save event2");

        // Load events
        let events = bridge.get_events("order-1").await.expect("get events");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "OrderPlaced");
        assert_eq!(events[0].version, 1);
        assert_eq!(events[1].event_type, "OrderShipped");
        assert_eq!(events[1].version, 2);

        // Empty aggregate
        let no_events = bridge.get_events("missing").await.expect("get missing");
        assert!(no_events.is_empty());

        // Projections
        bridge
            .upsert_projection("proj:1", &serde_json::json!({"name": "test"}))
            .await
            .expect("upsert projection");

        let loaded: Option<serde_json::Value> = bridge
            .get_projection("proj:1")
            .await
            .expect("get projection");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap()["name"], "test");

        // Missing projection
        let missing: Option<serde_json::Value> = bridge
            .get_projection("nonexistent")
            .await
            .expect("get missing projection");
        assert!(missing.is_none());

        // Overwrite projection
        bridge
            .upsert_projection("proj:1", &serde_json::json!({"name": "updated"}))
            .await
            .expect("upsert again");

        let loaded2: Option<serde_json::Value> = bridge
            .get_projection("proj:1")
            .await
            .expect("get after update");
        assert_eq!(loaded2.unwrap()["name"], "updated");
    }
}
