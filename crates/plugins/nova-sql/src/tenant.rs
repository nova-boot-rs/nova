use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

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

/// Resolves which column stores the tenant id for a given table.
pub trait TenantColumnResolver: Send + Sync + 'static {
    fn tenant_column_for_table<'a>(&'a self, table: &str) -> Option<&'a str>;
}

/// Error type used by tenant-column scoping utilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantColumnError {
    MissingTenantColumn { table: String },
    MissingTenant,
    TenantMismatch { expected: String, found: String },
}

impl fmt::Display for TenantColumnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTenantColumn { table } => {
                write!(f, "no tenant column configured for table '{table}'")
            }
            Self::MissingTenant => write!(f, "tenant value is missing"),
            Self::TenantMismatch { expected, found } => {
                write!(f, "tenant mismatch: expected '{expected}', found '{found}'")
            }
        }
    }
}

impl std::error::Error for TenantColumnError {}

/// Default resolver backed by an in-memory map with optional fallback column.
#[derive(Debug, Clone, Default)]
pub struct StaticTenantColumnResolver {
    per_table: HashMap<String, String>,
    default_column: Option<String>,
}

impl StaticTenantColumnResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_default_column(mut self, column: impl Into<String>) -> Self {
        self.default_column = Some(column.into());
        self
    }

    pub fn with_table_column(mut self, table: impl Into<String>, column: impl Into<String>) -> Self {
        self.per_table.insert(table.into(), column.into());
        self
    }
}

impl TenantColumnResolver for StaticTenantColumnResolver {
    fn tenant_column_for_table<'a>(&'a self, table: &str) -> Option<&'a str> {
        self.per_table
            .get(table)
            .map(String::as_str)
            .or(self.default_column.as_deref())
    }
}
