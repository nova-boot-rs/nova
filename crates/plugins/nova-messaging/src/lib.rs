use async_trait::async_trait;
use futures_util::StreamExt;
use nova_core::{NovaPlugin, async_trait as nova_async_trait, axum::Extension, axum::Router};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};

#[derive(Debug)]
pub enum MessagingError {
    Backend(String),
    Serialization(String),
    Handler(String),
    NotImplemented(&'static str),
}

impl fmt::Display for MessagingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "backend error: {msg}"),
            Self::Serialization(msg) => write!(f, "serialization error: {msg}"),
            Self::Handler(msg) => write!(f, "handler error: {msg}"),
            Self::NotImplemented(msg) => write!(f, "not implemented: {msg}"),
        }
    }
}

impl std::error::Error for MessagingError {}

/// Standard event envelope used by all messaging backends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: String,
    pub topic: String,
    pub event_type: String,
    pub payload: JsonValue,
    pub key: Option<String>,
    pub headers: HashMap<String, String>,
    pub attempts: u32,
    pub timestamp_ms: u64,
}

impl EventEnvelope {
    pub fn new(
        id: impl Into<String>,
        topic: impl Into<String>,
        event_type: impl Into<String>,
        payload: JsonValue,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            id: id.into(),
            topic: topic.into(),
            event_type: event_type.into(),
            payload,
            key: None,
            headers: HashMap::new(),
            attempts: 0,
            timestamp_ms,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn to_payload<T: DeserializeOwned>(&self) -> Result<T, MessagingError> {
        serde_json::from_value(self.payload.clone())
            .map_err(|e| MessagingError::Serialization(e.to_string()))
    }
}

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

pub struct KafkaBroker {
    pub brokers: String,
    pub client_id: String,
    producer: Mutex<Option<FutureProducer>>,
}

impl KafkaBroker {
    pub fn new(brokers: Vec<String>, client_id: impl Into<String>) -> Self {
        Self {
            brokers: brokers.join(","),
            client_id: client_id.into(),
            producer: Mutex::new(None),
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
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", &self.brokers)
            .set("group.id", format!("{}-poll", self.client_id))
            .set("auto.offset.reset", "latest")
            .set("enable.auto.commit", "true")
            .set("session.timeout.ms", "6000")
            .create()
            .map_err(|e| MessagingError::Backend(e.to_string()))?;

        consumer
            .subscribe(&[topic])
            .map_err(|e| MessagingError::Backend(e.to_string()))?;

        let mut out = Vec::new();
        let stream = consumer.stream();
        futures_util::pin_mut!(stream);

        for _ in 0..max_messages {
            match tokio::time::timeout(Duration::from_millis(100), stream.next()).await {
                Ok(Some(Ok(msg))) => {
                    let payload = msg
                        .payload()
                        .ok_or_else(|| MessagingError::Backend("empty payload".to_string()))?;
                    let env = serde_json::from_slice::<EventEnvelope>(payload)
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

pub struct NatsBroker {
    pub server_url: String,
    pub poll_timeout_ms: u64,
}

impl NatsBroker {
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            poll_timeout_ms: 100,
        }
    }

    fn dlq_topic(source_topic: &str) -> String {
        format!("{source_topic}.dlq")
    }

    async fn connect(&self) -> Result<async_nats::Client, MessagingError> {
        async_nats::connect(&self.server_url)
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))
    }
}

#[async_trait]
impl MessageBroker for NatsBroker {
    async fn publish(&self, envelope: EventEnvelope) -> Result<(), MessagingError> {
        let client = self.connect().await?;
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
        let client = self.connect().await?;
        let mut sub = client
            .subscribe(topic.to_string())
            .await
            .map_err(|e| MessagingError::Backend(e.to_string()))?;

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
                Ok(None) => break,
                Err(_) => break,
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

#[derive(Clone)]
pub struct NovaMessaging {
    broker: Arc<dyn MessageBroker>,
}

impl NovaMessaging {
    pub fn new(broker: Arc<dyn MessageBroker>) -> Self {
        Self { broker }
    }

    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemoryBroker::default()))
    }

    pub fn kafka(brokers: Vec<String>, client_id: impl Into<String>) -> Self {
        Self::new(Arc::new(KafkaBroker::new(brokers, client_id)))
    }

    pub fn rabbitmq(amqp_url: impl Into<String>) -> Self {
        Self::new(Arc::new(RabbitMqBroker::new(amqp_url)))
    }

    pub fn nats(server_url: impl Into<String>) -> Self {
        Self::new(Arc::new(NatsBroker::new(server_url)))
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
        F: Fn(EventEnvelope) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), MessagingError>> + Send,
    {
        let messages = self.broker.poll(topic, max_messages).await?;
        let mut ok = 0usize;

        for env in messages {
            match handler(env.clone()).await {
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

#[nova_async_trait]
impl NovaPlugin for NovaMessaging {
    fn name(&self) -> &'static str {
        "NovaMessaging"
    }

    async fn on_init(&self) {
        println!("📨 Initializing Messaging Plugin...");
    }

    fn extend_router(&self, router: Router<()>) -> Router<()> {
        router.layer(Extension(self.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct UserCreated {
        id: String,
        email: String,
    }

    #[tokio::test]
    async fn envelope_roundtrip_payload_deserialization() {
        let env = EventEnvelope::new(
            "e-1",
            "users",
            "user.created",
            serde_json::json!({"id":"u1", "email":"u1@nova.rs"}),
        );

        let parsed: UserCreated = env.to_payload().expect("payload should deserialize");
        assert_eq!(parsed.id, "u1");
        assert_eq!(parsed.email, "u1@nova.rs");
    }

    #[tokio::test]
    async fn in_memory_publish_and_poll_json() {
        let messaging = NovaMessaging::in_memory();
        let event = UserCreated {
            id: "u1".to_string(),
            email: "u1@nova.rs".to_string(),
        };

        messaging
            .publish_json("e-1", "users", "user.created", &event)
            .await
            .expect("publish should succeed");

        let out: Vec<UserCreated> = messaging
            .poll_json("users", 10)
            .await
            .expect("poll should succeed");

        assert_eq!(out, vec![event]);
    }

    #[tokio::test]
    async fn failed_handler_routes_to_dlq() {
        let messaging = NovaMessaging::in_memory();
        messaging
            .publish_json(
                "e-1",
                "users",
                "user.created",
                &serde_json::json!({"id":"u1"}),
            )
            .await
            .expect("publish should succeed");

        let processed = messaging
            .process_with_dlq("users", 10, |_env| async {
                Err(MessagingError::Handler("boom".to_string()))
            })
            .await
            .expect("processor should not fail hard");
        assert_eq!(processed, 0);

        let dlq = messaging
            .poll_dlq("users", 10)
            .await
            .expect("dlq poll should succeed");
        assert_eq!(dlq.len(), 1);
        assert_eq!(
            dlq[0].headers.get("x-source-topic"),
            Some(&"users".to_string())
        );
        assert_eq!(
            dlq[0].headers.get("x-dlq-reason"),
            Some(&"handler error: boom".to_string())
        );
        assert_eq!(dlq[0].attempts, 1);
    }

    #[tokio::test]
    async fn kafka_without_server_returns_backend_error() {
        let kafka = NovaMessaging::kafka(vec!["127.0.0.1:65535".to_string()], "nova");

        let e = EventEnvelope::new("e-1", "users", "user.created", serde_json::json!({}));

        let r = kafka.broker.publish(e).await;

        assert!(matches!(r, Err(MessagingError::Backend(_))));
    }

    #[tokio::test]
    async fn nats_without_server_returns_backend_error() {
        let nats = NovaMessaging::nats("nats://127.0.0.1:65535");
        let e = EventEnvelope::new("e-1", "users", "user.created", serde_json::json!({}));
        let r = nats.broker.publish(e).await;
        assert!(matches!(r, Err(MessagingError::Backend(_))));
    }

    #[tokio::test]
    async fn rabbitmq_without_server_returns_backend_error() {
        let rabbit = NovaMessaging::rabbitmq("amqp://127.0.0.1:65535");
        let e = EventEnvelope::new("e-1", "users", "user.created", serde_json::json!({}));
        let r = rabbit.broker.publish(e).await;
        assert!(matches!(r, Err(MessagingError::Backend(_))));
    }
}
