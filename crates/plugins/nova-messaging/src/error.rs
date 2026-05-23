use std::fmt;

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
