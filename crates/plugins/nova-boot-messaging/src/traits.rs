use crate::{envelope::EventEnvelope, error::MessagingError};
use async_trait::async_trait;

/// Adapter trait for messaging backends.
///
/// Implement this trait to support a new broker (Kafka, NATS, RabbitMQ,
/// in-memory). The trait covers publishing, polling, and DLQ operations.
#[async_trait]
pub trait MessageBroker: Send + Sync {
    /// Publish an envelope to the broker.
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), MessagingError>;

    /// Poll up to `max_messages` from `topic`.
    async fn poll(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError>;

    /// Publish a message to the dead-letter queue for `source_topic`.
    async fn publish_dlq(
        &self,
        source_topic: &str,
        envelope: EventEnvelope,
        reason: &str,
    ) -> Result<(), MessagingError>;

    /// Poll messages from the DLQ for `source_topic`.
    async fn poll_dlq(
        &self,
        source_topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError>;
}
