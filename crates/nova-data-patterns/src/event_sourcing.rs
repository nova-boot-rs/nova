//! Event sourcing helpers and in-memory event store.
//!
//! Exposes the `Aggregate` trait, `EventStore` trait, and a generic
//! `EventSourcedRepository` that can rebuild aggregates from event streams and
//! persist new domain events. Includes an `InMemoryEventStore` for tests.
use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::cqrs::StoredEvent;
use crate::error::CqrsError;

// ---------------------------------------------------------------------------
// Aggregate – rebuild state from an event stream
// ---------------------------------------------------------------------------

#[async_trait]
pub trait Aggregate: Default + Send + Sync {
    type Event: Clone + Send + Sync;
    type Error: std::error::Error + Send + Sync;

    fn apply(&mut self, event: &Self::Event);
    fn version(&self) -> u32;
    fn set_version(&mut self, version: u32);
}

// ---------------------------------------------------------------------------
// EventStore – append / load with optimistic concurrency
// ---------------------------------------------------------------------------

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append_events(
        &self,
        aggregate_id: &str,
        events: &[StoredEvent],
        expected_version: u32,
    ) -> Result<(), CqrsError>;

    async fn load_events(&self, aggregate_id: &str) -> Result<Vec<StoredEvent>, CqrsError>;
}

// ---------------------------------------------------------------------------
// EventSourcedRepository – generic loader / saver for any Aggregate
// ---------------------------------------------------------------------------

pub struct EventSourcedRepository<A: Aggregate, S: EventStore> {
    store: S,
    _marker: std::marker::PhantomData<A>,
}

impl<A, S> EventSourcedRepository<A, S>
where
    A: Aggregate + 'static,
    S: EventStore,
{
    pub fn new(store: S) -> Self {
        Self {
            store,
            _marker: std::marker::PhantomData,
        }
    }

    /// Replay all stored events for `aggregate_id` and return the rebuilt
    /// aggregate. Returns `None` when no events exist.
    pub async fn load(&self, aggregate_id: &str) -> Result<Option<A>, CqrsError>
    where
        A::Event: DeserializeOwned,
    {
        let stored_events = self.store.load_events(aggregate_id).await?;
        if stored_events.is_empty() {
            return Ok(None);
        }

        let mut aggregate = A::default();
        for stored in &stored_events {
            let event: A::Event = serde_json::from_value(stored.payload.clone())
                .map_err(|e| CqrsError::Serialization(e.to_string()))?;
            aggregate.apply(&event);
        }

        if let Some(last) = stored_events.last() {
            aggregate.set_version(last.version);
        }

        info!(
            aggregate_id = %aggregate_id,
            event_count = %stored_events.len(),
            version = %aggregate.version(),
            "rebuilt aggregate from event stream"
        );

        Ok(Some(aggregate))
    }

    /// Persist new domain events. Uses optimistic concurrency: checks that the
    /// current event stream is at `expected_version` before appending.
    pub async fn save(
        &self,
        aggregate_id: &str,
        events: &[A::Event],
        expected_version: u32,
    ) -> Result<(), CqrsError>
    where
        A::Event: Serialize,
    {
        if events.is_empty() {
            return Ok(());
        }

        let stored: Vec<StoredEvent> = events
            .iter()
            .enumerate()
            .map(|(i, event)| {
                let payload = serde_json::to_value(event).unwrap_or(serde_json::Value::Null);
                StoredEvent::new(
                    aggregate_id,
                    std::any::type_name::<A::Event>(),
                    payload,
                    expected_version + 1 + i as u32,
                )
            })
            .collect();

        info!(
            aggregate_id = %aggregate_id,
            count = %stored.len(),
            expected_version = %expected_version,
            "persisting domain events"
        );

        self.store
            .append_events(aggregate_id, &stored, expected_version)
            .await
    }
}

// ---------------------------------------------------------------------------
// In-memory EventStore (dev / test)
// ---------------------------------------------------------------------------

pub struct InMemoryEventStore {
    events: Arc<RwLock<HashMap<String, Vec<StoredEvent>>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn append_events(
        &self,
        aggregate_id: &str,
        events: &[StoredEvent],
        expected_version: u32,
    ) -> Result<(), CqrsError> {
        let mut map = self.events.write().await;
        let stream = map.entry(aggregate_id.to_string()).or_default();
        let current_version = stream.last().map(|e| e.version).unwrap_or(0);

        if current_version != expected_version {
            return Err(CqrsError::Concurrency(format!(
                "expected version {expected_version} but current version is {current_version} for aggregate '{aggregate_id}'",
            )));
        }

        for event in events {
            info!(
                aggregate_id = %event.aggregate_id,
                event_type = %event.event_type,
                version = %event.version,
                "in-memory event store: appending event"
            );
            stream.push(event.clone());
        }

        Ok(())
    }

    async fn load_events(&self, aggregate_id: &str) -> Result<Vec<StoredEvent>, CqrsError> {
        let map = self.events.read().await;
        Ok(map.get(aggregate_id).cloned().unwrap_or_default())
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
    enum OrderEvent {
        OrderPlaced { order_id: String, total: f64 },
        OrderShipped { tracking: String },
    }

    #[derive(Debug, Default, Clone, PartialEq)]
    struct Order {
        id: String,
        total: f64,
        tracking: String,
        version: u32,
    }

    #[async_trait]
    impl Aggregate for Order {
        type Event = OrderEvent;
        type Error = std::convert::Infallible;

        fn apply(&mut self, event: &Self::Event) {
            match event {
                OrderEvent::OrderPlaced { order_id, total } => {
                    self.id = order_id.clone();
                    self.total = *total;
                }
                OrderEvent::OrderShipped { tracking } => {
                    self.tracking = tracking.clone();
                }
            }
        }

        fn version(&self) -> u32 {
            self.version
        }

        fn set_version(&mut self, version: u32) {
            self.version = version;
        }
    }

    fn make_repo() -> EventSourcedRepository<Order, InMemoryEventStore> {
        let store = InMemoryEventStore::new();
        EventSourcedRepository::new(store)
    }

    #[tokio::test]
    async fn save_and_reload_aggregate() {
        let repo = make_repo();

        let events = vec![
            OrderEvent::OrderPlaced {
                order_id: "ord-1".to_string(),
                total: 99.99,
            },
            OrderEvent::OrderShipped {
                tracking: "TRK123".to_string(),
            },
        ];

        repo.save("ord-1", &events, 0)
            .await
            .expect("save should succeed");

        let loaded = repo.load("ord-1").await.expect("load should succeed");
        assert!(loaded.is_some());
        let order = loaded.unwrap();
        assert_eq!(order.id, "ord-1");
        assert_eq!(order.total, 99.99);
        assert_eq!(order.tracking, "TRK123");
        assert_eq!(order.version, 2);
    }

    #[tokio::test]
    async fn load_nonexistent_aggregate_returns_none() {
        let repo = make_repo();
        let loaded = repo.load("missing").await.expect("load should succeed");
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn concurrency_conflict_on_version_mismatch() {
        let repo = make_repo();

        let events = vec![OrderEvent::OrderPlaced {
            order_id: "ord-2".to_string(),
            total: 50.0,
        }];
        repo.save("ord-2", &events, 0)
            .await
            .expect("first save succeeds");

        // Trying to save from version 0 again causes conflict
        let result = repo.save("ord-2", &events, 0).await;
        assert!(result.is_err());
        match result {
            Err(CqrsError::Concurrency(msg)) => {
                assert!(msg.contains("ord-2"));
            }
            _ => panic!("expected Concurrency error"),
        }
    }

    #[tokio::test]
    async fn save_empty_events_is_noop() {
        let repo = make_repo();
        repo.save("ord-3", &[] as &[OrderEvent], 0)
            .await
            .expect("empty save should succeed");

        let loaded = repo.load("ord-3").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn multiple_saves_append_to_stream() {
        let repo = make_repo();

        let placed = vec![OrderEvent::OrderPlaced {
            order_id: "ord-4".to_string(),
            total: 25.0,
        }];
        repo.save("ord-4", &placed, 0).await.unwrap();

        let shipped = vec![OrderEvent::OrderShipped {
            tracking: "TRK456".to_string(),
        }];
        repo.save("ord-4", &shipped, 1).await.unwrap();

        let loaded = repo.load("ord-4").await.unwrap().unwrap();
        assert_eq!(loaded.version, 2);
        assert_eq!(loaded.tracking, "TRK456");
    }
}
