use crate::{envelope::EventEnvelope, error::MessagingError, traits::MessageBroker};
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct InMemoryBroker {
    queues: Arc<RwLock<HashMap<String, VecDeque<EventEnvelope>>>>,
    dlq: Arc<RwLock<HashMap<String, VecDeque<EventEnvelope>>>>,
}

#[async_trait]
impl MessageBroker for InMemoryBroker {
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), MessagingError> {
        let topic = envelope.topic.clone();
        let mut queues = self.queues.write().await;
        queues.entry(topic).or_default().push_back(envelope);
        Ok(())
    }

    async fn poll(
        &self,
        topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        let mut queues = self.queues.write().await;
        let queue = queues.entry(topic.to_string()).or_default();

        let mut out = Vec::new();
        for _ in 0..max_messages {
            if let Some(msg) = queue.pop_front() {
                out.push(msg);
            } else {
                break;
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
        envelope = envelope
            .with_header("x-dlq-reason", reason)
            .with_header("x-source-topic", source_topic);
        envelope.attempts = envelope.attempts.saturating_add(1);

        let mut dlq = self.dlq.write().await;
        dlq.entry(source_topic.to_string())
            .or_default()
            .push_back(envelope);
        Ok(())
    }

    async fn poll_dlq(
        &self,
        source_topic: &str,
        max_messages: usize,
    ) -> Result<Vec<EventEnvelope>, MessagingError> {
        let mut dlq = self.dlq.write().await;
        let queue = dlq.entry(source_topic.to_string()).or_default();

        let mut out = Vec::new();
        for _ in 0..max_messages {
            if let Some(msg) = queue.pop_front() {
                out.push(msg);
            } else {
                break;
            }
        }
        Ok(out)
    }
}
