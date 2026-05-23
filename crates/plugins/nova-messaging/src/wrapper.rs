use crate::{envelope::EventEnvelope, error::MessagingError, memory::InMemoryBroker, traits::MessageBroker};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct NovaMessaging {
    pub(crate) broker: Arc<dyn MessageBroker>,
}

impl NovaMessaging {
    pub fn new(broker: Arc<dyn MessageBroker>) -> Self {
        Self { broker }
    }

    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemoryBroker::default()))
    }

    pub fn kafka(brokers: Vec<String>, client_id: impl Into<String>) -> Self {
        Self::new(Arc::new(crate::kafka::KafkaBroker::new(brokers, client_id)))
    }

    pub fn rabbitmq(amqp_url: impl Into<String>) -> Self {
        Self::new(Arc::new(crate::rabbitmq::RabbitMqBroker::new(amqp_url)))
    }

    pub fn nats(server_url: impl Into<String>) -> Self {
        Self::new(Arc::new(crate::nats::NatsBroker::new(server_url)))
    }

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

    pub async fn publish_envelope(&self, envelope: EventEnvelope) -> Result<(), MessagingError> {
        self.broker.publish(envelope).await
    }

    pub async fn poll(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        self.broker.poll(topic, max_messages).await
    }

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

    pub async fn poll_dlq(
        &self,
        source_topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        self.broker.poll_dlq(source_topic, max_messages).await
    }

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