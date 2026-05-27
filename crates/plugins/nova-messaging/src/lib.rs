//! Nova messaging abstractions and broker implementations.

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
