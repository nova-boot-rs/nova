use std::fmt;

#[derive(Debug, Clone)]
pub enum CqrsError {
    Backend(String),
    Serialization(String),
    Concurrency(String),
}

impl fmt::Display for CqrsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "CQRS backend error: {msg}"),
            Self::Serialization(msg) => write!(f, "CQRS serialization error: {msg}"),
            Self::Concurrency(msg) => write!(f, "CQRS concurrency error: {msg}"),
        }
    }
}

impl std::error::Error for CqrsError {}

impl From<serde_json::Error> for CqrsError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum SagaError {
    StepFailed { step: String, reason: String },
    CompensationFailed { step: String, reason: String },
    Timeout { step: String },
    Aborted(String),
}

impl fmt::Display for SagaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StepFailed { step, reason } => {
                write!(f, "saga step '{step}' failed: {reason}")
            }
            Self::CompensationFailed { step, reason } => {
                write!(f, "saga compensation for step '{step}' failed: {reason}")
            }
            Self::Timeout { step } => {
                write!(f, "saga step '{step}' timed out")
            }
            Self::Aborted(msg) => write!(f, "saga aborted: {msg}"),
        }
    }
}

impl std::error::Error for SagaError {}
