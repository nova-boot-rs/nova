use std::fmt;

/// Errors returned by CQRS stores and operations.
#[derive(Debug, Clone)]
pub enum CqrsError {
    /// Backend-specific error.
    Backend(String),
    /// Serialization/deserialization error.
    Serialization(String),
    /// Optimistic concurrency violation.
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

/// Errors used by the Saga orchestration helpers.
#[derive(Debug, Clone)]
pub enum SagaError {
    /// A saga step failed with a specific reason.
    StepFailed { step: String, reason: String },
    /// Compensation for a step failed.
    CompensationFailed { step: String, reason: String },
    /// Step timed out.
    Timeout { step: String },
    /// Generic abort with message.
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
