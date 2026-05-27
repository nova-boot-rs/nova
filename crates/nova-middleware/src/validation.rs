//! Basic validation helpers and traits for request payloads.
//!
//! Implement `NovaValidate` for your request types and use `validate_request`
//! in controllers to convert validation results into `NovaResult<()>`.

use nova_core::{NovaError, NovaResult};

/// Collector for validation errors that can be converted into a `NovaResult`.
#[derive(Debug, Clone, Default)]
pub struct ValidationErrors {
    errors: Vec<String>,
}

impl ValidationErrors {
    /// Create an empty validation errors collector.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add an error message to the collector.
    pub fn push(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    /// Returns true when no errors were recorded.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Convert into a `NovaResult<()>`, returning `Ok(())` when empty or a
    /// `NovaError::ValidationError` otherwise.
    pub fn into_result(self) -> NovaResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(NovaError::ValidationError(self.errors.join("; ")))
        }
    }
}

/// Trait to implement on request payloads to perform validation.
pub trait NovaValidate {
    fn validate(&self) -> Result<(), ValidationErrors>;
}

/// Convenience wrapper that runs validation and converts into `NovaResult<()>`.
pub fn validate_request<T: NovaValidate>(value: &T) -> NovaResult<()> {
    value
        .validate()
        .map_err(|errs| NovaError::ValidationError(errs.errors.join("; ")))
}

/// Helper that returns an error message when a string is blank.
pub fn required_string(field: &str, value: &str) -> Option<String> {
    if value.trim().is_empty() {
        Some(format!("{field} is required"))
    } else {
        None
    }
}

/// Validate minimum character length for a string field.
pub fn min_length(field: &str, value: &str, min: usize) -> Option<String> {
    if value.chars().count() < min {
        Some(format!("{field} must be at least {min} characters"))
    } else {
        None
    }
}

/// Validate maximum character length for a string field.
pub fn max_length(field: &str, value: &str, max: usize) -> Option<String> {
    if value.chars().count() > max {
        Some(format!("{field} must be at most {max} characters"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Payload {
        content: String,
    }

    impl NovaValidate for Payload {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();

            if let Some(err) = required_string("content", &self.content) {
                errors.push(err);
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }

    #[test]
    fn validation_fails_for_blank_content() {
        let payload = Payload {
            content: "   ".to_string(),
        };

        let result = validate_request(&payload);
        assert!(result.is_err());
    }

    #[test]
    fn validation_succeeds_for_non_blank_content() {
        let payload = Payload {
            content: "hello".to_string(),
        };

        let result = validate_request(&payload);
        assert!(result.is_ok());
    }
}
