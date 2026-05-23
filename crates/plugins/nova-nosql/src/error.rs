use std::fmt;

#[derive(Debug)]
pub enum NoSqlError {
    Backend(String),
    Serialization(String),
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