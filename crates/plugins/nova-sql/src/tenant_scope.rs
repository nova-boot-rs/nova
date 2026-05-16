use super::tenant::Tenant;
use sea_orm::DatabaseConnection;

/// Wraps a database connection + tenant for scoped queries.
#[derive(Clone)]
pub struct TenantScope {
    pub db: DatabaseConnection,
    pub tenant: Tenant,
}

impl TenantScope {
    pub fn new(db: DatabaseConnection, tenant: Tenant) -> Self {
        Self { db, tenant }
    }
    
    /// Get the tenant ID for query filtering.
    pub fn tenant_id(&self) -> &str {
        self.tenant.as_str()
    }
}
