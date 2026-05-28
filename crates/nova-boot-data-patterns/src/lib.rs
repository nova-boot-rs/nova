//! Cross-cutting data patterns for the Nova framework:
//! CQRS, Event Sourcing, and Saga orchestration.

pub mod cqrs;
pub mod error;
pub mod event_sourcing;
pub mod saga;

#[cfg(any(
    feature = "sql-bridge",
    feature = "nosql-bridge",
    feature = "messaging-bridge"
))]
pub mod bridges;

// Re-exports
pub use cqrs::{
    Command, CommandStore, InMemoryCqrsStore, Query, QueryStore, QueryStoreExt, StoredEvent,
};
pub use error::{CqrsError, SagaError};
pub use event_sourcing::{Aggregate, EventSourcedRepository, EventStore, InMemoryEventStore};
pub use saga::{Saga, SagaExecution, SagaExecutionStatus, SagaStep};

#[cfg(feature = "messaging-bridge")]
pub use saga::SagaCoordinator;

#[cfg(feature = "redis-store")]
pub use cqrs::RedisQueryStore;

#[cfg(feature = "sql-bridge")]
pub use bridges::SqlCqrsBridge;

#[cfg(feature = "nosql-bridge")]
pub use bridges::NoSqlQueryStore;

#[cfg(feature = "messaging-bridge")]
pub use bridges::MessagingSagaBus;
