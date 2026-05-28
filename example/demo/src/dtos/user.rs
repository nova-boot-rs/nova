use nova_boot::{Deserialize, NovaRequest, NovaResponse, Serialize};
use nova_boot_middleware::{NovaValidate, ValidationErrors, max_length, required_string};

// Response DTO uses NovaResponse so the demo shows the framework's typed
// response-deriving path.
#[derive(Serialize, Clone, NovaResponse)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
}

// Request DTO uses NovaRequest so it can be validated at the handler edge.
#[derive(Deserialize, NovaRequest)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
}

impl NovaValidate for CreateUser {
    fn validate(&self) -> Result<(), ValidationErrors> {
        // Keep validation rules close to the DTO to make the demo readable.
        let mut errors = ValidationErrors::new();

        if let Some(err) = required_string("username", &self.username) {
            errors.push(err);
        }

        if let Some(err) = required_string("email", &self.email) {
            errors.push(err);
        }

        if let Some(err) = max_length("username", &self.username, 64) {
            errors.push(err);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
