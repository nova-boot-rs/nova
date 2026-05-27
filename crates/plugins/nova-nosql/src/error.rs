use std::fmt;

/// Error type for NoSQL adapters and operations.
#[derive(Debug)]
pub enum NoSqlError {
    /// Backend-specific error with a human-readable message.
    Backend(String),
    /// Serialization/deserialization error.
    Serialization(String),
    /// Placeholder for features not yet implemented.
    NotImplemented(&'static str),
}

impl fmt::Display for NoSqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "backend error: {msg}"),
            Self::Serialization(msg) => write!(f, "serialization error: {msg}"),
            Self::NotImplemented(msg) => write!(f, "not implemented: {msg}"),
        }
    }
}

impl std::error::Error for NoSqlError {}
