use crate::{
    envelope::EventEnvelope, error::MessagingError, memory::InMemoryBroker, traits::MessageBroker,
};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

/// High-level messaging wrapper exposing publish and poll helpers.
///
/// `NovaMessaging` composes a backend `MessageBroker` implementation (Kafka,
/// NATS, RabbitMQ, in-memory) and provides ergonomic convenience methods used by
/// handlers and background jobs.
#[derive(Clone)]
pub struct NovaMessaging {
    pub(crate) broker: Arc<dyn MessageBroker>,
}

impl NovaMessaging {
    /// Construct with a custom broker implementation.
    pub fn new(broker: Arc<dyn MessageBroker>) -> Self {
        Self { broker }
    }

    /// In-memory broker useful for tests and local development.
    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemoryBroker::default()))
    }

    /// Kafka-backed messaging.
    pub fn kafka(brokers: Vec<String>, client_id: impl Into<String>) -> Self {
        Self::new(Arc::new(crate::kafka::KafkaBroker::new(brokers, client_id)))
    }

    /// RabbitMQ-backed messaging.
    pub fn rabbitmq(amqp_url: impl Into<String>) -> Self {
        Self::new(Arc::new(crate::rabbitmq::RabbitMqBroker::new(amqp_url)))
    }

    /// NATS-backed messaging.
    pub fn nats(server_url: impl Into<String>) -> Self {
        Self::new(Arc::new(crate::nats::NatsBroker::new(server_url)))
    }

    /// Publish a JSON-serializable payload as an `EventEnvelope`.
    pub async fn publish_json<T: Serialize>(
        &self,
        id: impl Into<String>,
        topic: impl Into<String>,
        event_type: impl Into<String>,
        payload: &T,
    ) -> Result<(), MessagingError> {
        let payload_json = serde_json::to_value(payload)
            .map_err(|e| MessagingError::Serialization(e.to_string()))?;
        let envelope = EventEnvelope::new(id, topic, event_type, payload_json);
        self.broker.publish(envelope).await
    }

    /// Publish a fully-formed envelope.
    pub async fn publish_envelope(&self, envelope: EventEnvelope) -> Result<(), MessagingError> {
        self.broker.publish(envelope).await
    }

    /// Poll up to `max_messages` from `topic`.
    pub async fn poll(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        self.broker.poll(topic, max_messages).await
    }

    /// Poll and deserialize messages to `T`.
    pub async fn poll_json<T: DeserializeOwned>(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<T>, MessagingError> {
        let envelopes = self.broker.poll(topic, max_messages).await?;
        envelopes
            .into_iter()
            .map(|env| env.to_payload::<T>())
            .collect::<Result<Vec<_>, _>>()
    }

    /// Process messages with a handler; failed items are sent to a DLQ.
    pub async fn process_with_dlq<F, Fut>(
        &self,
        topic: &str,
        max_messages: usize,
        handler: F,
    ) -> Result<usize, MessagingError>
    where
        F: Fn(&EventEnvelope) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), MessagingError>> + Send,
    {
        let messages = self.broker.poll(topic, max_messages).await?;
        let mut ok = 0usize;

        for env in messages {
            match handler(&env).await {
                Ok(()) => ok += 1,
                Err(err) => {
                    self.broker
                        .publish_dlq(topic, env, &err.to_string())
                        .await?;
                }
            }
        }
        Ok(ok)
    }

    /// Poll messages from the DLQ for `source_topic`.
    pub async fn poll_dlq(
        &self,
        source_topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        self.broker.poll_dlq(source_topic, max_messages).await
    }

    /// Publish an envelope to the DLQ for `source_topic` with a reason.
    pub async fn publish_to_dlq(
        &self,
        source_topic: &str,
        envelope: EventEnvelope,
        reason: &str,
    ) -> Result<(), MessagingError> {
        self.broker
            .publish_dlq(source_topic, envelope, reason)
            .await
    }
}
