use crate::{envelope::EventEnvelope, error::MessagingError, traits::MessageBroker};
use async_trait::async_trait;
use futures_util::StreamExt;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message as RdkMessage;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Mutex;

pub struct KafkaBroker {
    pub brokers: String,
    pub client_id: String,
    pub poll_timeout_ms: u64,
    producer: Mutex<Option<FutureProducer>>,
    consumers: Mutex<HashMap<String, StreamConsumer>>,
}

impl KafkaBroker {
    pub fn new(brokers: Vec<String>, client_id: impl Into<String>) -> Self {
        Self {
            brokers: brokers.join(","),
            client_id: client_id.into(),
            poll_timeout_ms: 1000,
            producer: Mutex::new(None),
            consumers: Mutex::new(HashMap::new()),
        }
    }

    fn dlq_topic(source_topic: &str) -> String {
        format!("{source_topic}.dlq")
    }

    async fn get_or_create_producer(&self) -> Result<FutureProducer, MessagingError> {
        let mut guard = self.producer.lock().await;
        if let Some(ref producer) = *guard {
            return Ok(producer.clone());
        }
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &self.brokers)
            .set("client.id", &self.client_id)
            .set("message.timeout.ms", "5000")
            .create()
            .map_err(|e| MessagingError::Backend(e.to_string()))?;
        let cloned = producer.clone();
        *guard = Some(producer);
        Ok(cloned)
    }
}

#[async_trait]
impl MessageBroker for KafkaBroker {
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), MessagingError> {
        let producer = self.get_or_create_producer().await?;
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|e| MessagingError::Serialization(e.to_string()))?;

        let record = FutureRecord::<(), [u8]>::to(&envelope.topic).payload(&bytes);

        producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(e, _)| MessagingError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn poll(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        let mut guard = self.consumers.lock().await;
        let consumer = match guard.entry(topic.to_string()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let c: StreamConsumer = ClientConfig::new()
                    .set("bootstrap.servers", &self.brokers)
                    .set("group.id", format!("{}-{}", self.client_id, topic))
                    .set("auto.offset.reset", "earliest")
                    .set("enable.auto.commit", "true")
                    .set("session.timeout.ms", "6000")
                    .create()
                    .map_err(|e| MessagingError::Backend(e.to_string()))?;
                c.subscribe(&[topic])
                    .map_err(|e| MessagingError::Backend(e.to_string()))?;
                v.insert(c)
            }
        };

        let mut out = Vec::new();
        let stream = consumer.stream();
        futures_util::pin_mut!(stream);

        for _ in 0..max_messages {
            match tokio::time::timeout(Duration::from_millis(self.poll_timeout_ms), stream.next())
                .await
            {
                Ok(Some(Ok(msg))) => {
                    let payload = RdkMessage::payload(&msg)
                        .ok_or_else(|| MessagingError::Backend("empty payload".to_string()))?;
                    let env = serde_json::from_slice::<EventEnvelope>(payload)
                        .map_err(|e| MessagingError::Serialization(e.to_string()))?;
                    out.push(env);
                }
                Ok(Some(Err(e))) => {
                    return Err(MessagingError::Backend(e.to_string()));
                }
                _ => break,
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
        envelope.topic = Self::dlq_topic(source_topic);
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
        self.poll(&Self::dlq_topic(source_topic), max_messages)
            .await
    }
}