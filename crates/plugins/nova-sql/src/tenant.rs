use serde::{Deserialize, Serialize};
use std::fmt;
use async_trait::async_trait;

/// A tenant identifier — can be a UUID, slug, or opaque string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tenant(pub String);

impl Tenant {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Tenant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Trait for extracting tenant from a request.
#[async_trait]
pub trait TenantResolver: Send + Sync + 'static {
    /// Resolve tenant from an HTTP request headers/path/etc.
    async fn resolve(&self, headers: &axum::http::HeaderMap, path: &str) -> Option<Tenant>;
}
