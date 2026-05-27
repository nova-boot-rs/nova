//! Nova messaging abstractions and broker implementations.

// Embed example source in crate docs so users can view runnable examples
// directly in rustdoc.
#![doc = concat!("\n\n# Example: simple\n\n```rust\n", include_str!("../examples/simple.rs"), "\n```\n")]

pub mod envelope;
pub mod error;
pub mod kafka;
pub mod memory;
pub mod nats;
pub mod plugin;
pub mod rabbitmq;
pub mod traits;
pub mod wrapper;

pub use envelope::EventEnvelope;
pub use error::MessagingError;
pub use kafka::KafkaBroker;
pub use memory::InMemoryBroker;
pub use nats::NatsBroker;
pub use rabbitmq::RabbitMqBroker;
pub use traits::MessageBroker;
pub use wrapper::NovaMessaging;

#[cfg(test)]
mod tests;

mod extractors;
pub use extractors::NovaBus;
