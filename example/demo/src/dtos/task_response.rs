//! Task response DTO for task reads and creates.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: i64,
    pub title: String,
}
