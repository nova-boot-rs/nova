//! Create task request DTO.
//!
//! This shows Nova's request-deriving and validation flow: deserialize the
//! body, then validate the fields before the repository writes anything.

use nova_boot::NovaRequest;
use nova_boot_middleware::{NovaValidate, ValidationErrors, max_length, required_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, NovaRequest)]
pub struct CreateTask {
    pub title: String,
    pub payload: Option<serde_json::Value>,
}

impl NovaValidate for CreateTask {
    fn validate(&self) -> Result<(), ValidationErrors> {
        // Build validation errors field-by-field so the response is explicit.
        let mut errors = ValidationErrors::new();

        if let Some(err) = required_string("title", &self.title) {
            errors.push(err);
        }

        if let Some(err) = max_length("title", &self.title, 128) {
            errors.push(err);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
