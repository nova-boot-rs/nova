use crate::{envelope::EventEnvelope, error::MessagingError, traits::MessageBroker};
use async_trait::async_trait;

pub struct RabbitMqBroker {
    pub amqp_url: String,
}

impl RabbitMqBroker {
    pub fn new(amqp_url: impl Into<String>) -> Self {
        Self {
            amqp_url: amqp_url.into(),
        }
    }

    fn dlq_queue(source_topic: &str) -> String {
        format!("{source_topic}.dlq")
    }

    async fn connect(&self) -> Result<lapin::Connection, MessagingError> {
        lapin::Connection::connect(&self.amqp_url, lapin::ConnectionProperties::default())
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))
    }

    async fn ensure_queue(channel: &lapin::Channel, topic: &str) -> Result<(), MessagingError> {
        channel
            .queue_declare(
                topic,
                lapin::options::QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                lapin::types::FieldTable::default(),
            )
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl MessageBroker for RabbitMqBroker {
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), MessagingError> {
        let conn = self.connect().await?;
        let channel = conn
            .create_channel()
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))?;

        Self::ensure_queue(&channel, &envelope.topic).await?;

        let bytes = serde_json::to_vec(&envelope)
            .map_err(|e| MessagingError::Serialization(e.to_string()))?;

        channel
            .basic_publish(
                "",
                &envelope.topic,
                lapin::options::BasicPublishOptions::default(),
                &bytes,
                lapin::BasicProperties::default(),
            )
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn poll(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        let conn = self.connect().await?;
        let channel = conn
            .create_channel()
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))?;

        Self::ensure_queue(&channel, topic).await?;

        let mut out = Vec::new();
        for _ in 0..max_messages {
            let result = channel
                .basic_get(topic, lapin::options::BasicGetOptions { no_ack: true })
                .await
                .map_err(|e| MessagingError::Backend(e.to_string()))?;

            match result {
                Some(reply) => {
                    let env = serde_json::from_slice::<EventEnvelope>(&reply.delivery.data)
                        .map_err(|e| MessagingError::Serialization(e.to_string()))?;
                    out.push(env);
                }
                None => break,
            }
        }

        Ok(out)
    }

    async fn publish_dlq(
        &self,
        source_topic: &str,
        mut envelope: EventEnvelope,
        reason: &str,
    ) -> Result<(), MessagingError> {
        envelope.attempts = envelope.attempts.saturating_add(1);
        envelope.topic = Self::dlq_queue(source_topic);
        envelope
            .headers
            .insert("x-dlq-reason".to_string(), reason.to_string());
        envelope
            .headers
            .insert("x-source-topic".to_string(), source_topic.to_string());
        self.publish(envelope).await
    }

    async fn poll_dlq(
        &self,
        source_topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        self.poll(&Self::dlq_queue(source_topic), max_messages)
            .await
    }
}
