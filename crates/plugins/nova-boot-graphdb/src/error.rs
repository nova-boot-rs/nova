use std::fmt;

/// Error type emitted by graph store adapters.
#[derive(Debug)]
pub enum GraphDbError {
    /// Backend-specific error with details.
    Backend(String),
    /// Placeholder for unfinished features.
    NotImplemented(&'static str),
    /// Validation or input parsing error.
    InvalidInput(String),
    /// Serialization/deserialization error.
    Serialization(String),
}

impl fmt::Display for GraphDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "backend error: {msg}"),
            Self::NotImplemented(msg) => write!(f, "not implemented: {msg}"),
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::Serialization(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

impl std::error::Error for GraphDbError {}
