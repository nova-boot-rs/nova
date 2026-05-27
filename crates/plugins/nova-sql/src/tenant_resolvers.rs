use super::tenant::{Tenant, TenantResolver};
use async_trait::async_trait;
use axum::http::HeaderMap;

/// Resolves tenant from a header (e.g., `X-Tenant-Id: org-123`).
pub struct HeaderTenantResolver {
    pub header_name: String,
}

impl HeaderTenantResolver {
    pub fn new(header_name: impl Into<String>) -> Self {
        Self {
            header_name: header_name.into(),
        }
    }
}

#[async_trait]
impl TenantResolver for HeaderTenantResolver {
    async fn resolve(&self, headers: &HeaderMap, _path: &str) -> Option<Tenant> {
        headers
            .get(&self.header_name)
            .and_then(|v| v.to_str().ok())
            .map(|s| Tenant::new(s.to_string()))
    }
}

/// Resolves tenant from a JWT claim in Authorization header.
pub struct JwtTenantResolver {
    pub claim_name: String,
}

impl JwtTenantResolver {
    pub fn new(claim_name: impl Into<String>) -> Self {
        Self {
            claim_name: claim_name.into(),
        }
    }
}

#[async_trait]
impl TenantResolver for JwtTenantResolver {
    async fn resolve(&self, headers: &HeaderMap, _path: &str) -> Option<Tenant> {
        // In production, decode the JWT from the Authorization header
        // For now, this is a placeholder
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|_| Tenant::new("placeholder"))
    }
}

/// Resolves tenant from path parameter (e.g., `/org-123/users`).
pub struct PathTenantResolver {
    pub segment_index: usize,
}

impl PathTenantResolver {
    pub fn new(segment_index: usize) -> Self {
        Self { segment_index }
    }
}

#[async_trait]
impl TenantResolver for PathTenantResolver {
    async fn resolve(&self, _headers: &HeaderMap, path: &str) -> Option<Tenant> {
        path.split('/')
            .filter(|s| !s.is_empty())
            .nth(self.segment_index)
            .map(|s| Tenant::new(s.to_string()))
    }
}

/// Always returns the same tenant — for single-tenant apps or testing.
pub struct FixedTenantResolver {
    pub tenant: Tenant,
}

impl FixedTenantResolver {
    pub fn new(tenant: Tenant) -> Self {
        Self { tenant }
    }
}

#[async_trait]
impl TenantResolver for FixedTenantResolver {
    async fn resolve(&self, _headers: &HeaderMap, _path: &str) -> Option<Tenant> {
        Some(self.tenant.clone())
    }
}
