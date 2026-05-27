pub type NovaResult<T> = Result<T, ValidationErrors>;

#[derive(Debug, Clone, Default)]
pub struct ValidationErrors {
    errors: Vec<String>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn push(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn into_result(self) -> NovaResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

pub trait NovaValidate {
    fn validate(&self) -> Result<(), ValidationErrors>;
}

pub fn validate_request<T: NovaValidate>(value: &T) -> NovaResult<()> {
    value.validate()
}

pub fn required_string(field: &str, value: &str) -> Option<String> {
    if value.trim().is_empty() {
        Some(format!("{field} is required"))
    } else {
        None
    }
}

pub fn min_length(field: &str, value: &str, min: usize) -> Option<String> {
    if value.chars().count() < min {
        Some(format!("{field} must be at least {min} characters"))
    } else {
        None
    }
}

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
