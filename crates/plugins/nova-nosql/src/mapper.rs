use crate::error::NoSqlError;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;

/// Generic serde mapping helpers for NoSQL documents.
pub struct SerdeDocumentMapper;

impl SerdeDocumentMapper {
    pub fn to_value<T: Serialize>(value: &T) -> Result<JsonValue, NoSqlError> {
        serde_json::to_value(value).map_err(|e| NoSqlError::Serialization(e.to_string()))
    }

    pub fn from_value<T: DeserializeOwned>(value: JsonValue) -> Result<T, NoSqlError> {
        serde_json::from_value(value).map_err(|e| NoSqlError::Serialization(e.to_string()))
    }
}