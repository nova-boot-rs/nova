use crate::error::MessagingError;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

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
