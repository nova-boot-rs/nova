use crate::{envelope::EventEnvelope, error::MessagingError, traits::MessageBroker};
use async_trait::async_trait;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::time::Duration;
use tokio::sync::Mutex;

pub struct NatsBroker {
    pub server_url: String,
    pub poll_timeout_ms: u64,
    client: Mutex<Option<async_nats::Client>>,
    subscribers: Mutex<HashMap<String, async_nats::Subscriber>>,
}

impl NatsBroker {
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            poll_timeout_ms: 100,
            client: Mutex::new(None),
            subscribers: Mutex::new(HashMap::new()),
        }
    }

    fn dlq_topic(source_topic: &str) -> String {
        format!("{source_topic}.dlq")
    }

    async fn get_or_connect_client(&self) -> Result<async_nats::Client, MessagingError> {
        let mut guard = self.client.lock().await;
        if let Some(ref client) = *guard {
            return Ok(client.clone());
        }
        let client = async_nats::connect(&self.server_url)
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))?;
        let cloned = client.clone();
        *guard = Some(client);
        Ok(cloned)
    }
}

#[async_trait]
impl MessageBroker for NatsBroker {
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), MessagingError> {
        let client = self.get_or_connect_client().await?;
        let subject = envelope.topic.clone();
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|e| MessagingError::Serialization(e.to_string()))?;

        client
            .publish(subject, bytes.into())
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))
    }

    async fn poll(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        let mut guard = self.subscribers.lock().await;
        let sub = match guard.entry(topic.to_string()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let client = self.get_or_connect_client().await?;
                let s = client
                    .subscribe(topic.to_string())
                    .await
                    .map_err(|e| MessagingError::Backend(e.to_string()))?;
                v.insert(s)
            }
        };

        let mut out = Vec::new();
        for _ in 0..max_messages {
            match tokio::time::timeout(Duration::from_millis(self.poll_timeout_ms), sub.next())
                .await
            {
                Ok(Some(message)) => {
                    let env = serde_json::from_slice::<EventEnvelope>(message.payload.as_ref())
                        .map_err(|e| MessagingError::Serialization(e.to_string()))?;
                    out.push(env);
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
