use crate::{envelope::EventEnvelope, error::MessagingError};
use async_trait::async_trait;

#[async_trait]
pub trait MessageBroker: Send + Sync {
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), MessagingError>;
    async fn poll(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError>;
    async fn publish_dlq(
        &self,
        source_topic: &str,
        envelope: EventEnvelope,
        reason: &str,
    ) -> Result<(), MessagingError>;
    async fn poll_dlq(
        &self,
        source_topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError>;
}
