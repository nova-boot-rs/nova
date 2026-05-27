use super::tenant::{StaticTenantColumnResolver, Tenant, TenantColumnError, TenantColumnResolver};
use sea_orm::DatabaseConnection;
use sea_query::{Alias, DeleteStatement, Expr, SelectStatement, UpdateStatement};
use std::sync::Arc;

/// Wraps a database connection + tenant for scoped queries.
#[derive(Clone)]
pub struct TenantScope {
    pub db: DatabaseConnection,
    pub tenant: Tenant,
    resolver: Arc<dyn TenantColumnResolver>,
}

impl TenantScope {
    pub fn new(db: DatabaseConnection, tenant: Tenant) -> Self {
        Self {
            db,
            tenant,
            resolver: Arc::new(StaticTenantColumnResolver::new().with_default_column("tenant_id")),
        }
    }

    pub fn with_resolver(
        db: DatabaseConnection,
        tenant: Tenant,
        resolver: Arc<dyn TenantColumnResolver>,
    ) -> Self {
        Self {
            db,
            tenant,
            resolver,
        }
    }

    /// Get the tenant ID for query filtering.
    pub fn tenant_id(&self) -> &str {
        self.tenant.as_str()
    }

    /// Return the resolved tenant column for a table.
    pub fn tenant_column_for_table(&self, table: &str) -> Result<&str, TenantColumnError> {
        self.resolver.tenant_column_for_table(table).ok_or_else(|| {
            TenantColumnError::MissingTenantColumn {
                table: table.to_string(),
            }
        })
    }

    /// Apply tenant where-clause to a select statement.
    pub fn apply_select_scope(
        &self,
        table: &str,
        stmt: &mut SelectStatement,
    ) -> Result<(), TenantColumnError> {
        let tenant_column = self.tenant_column_for_table(table)?;
        stmt.and_where(
            Expr::col((Alias::new(table), Alias::new(tenant_column))).eq(self.tenant.as_str()),
        );
        Ok(())
    }

    /// Apply tenant where-clause to an update statement.
    pub fn apply_update_scope(
        &self,
        table: &str,
        stmt: &mut UpdateStatement,
    ) -> Result<(), TenantColumnError> {
        let tenant_column = self.tenant_column_for_table(table)?;
        stmt.and_where(
            Expr::col((Alias::new(table), Alias::new(tenant_column))).eq(self.tenant.as_str()),
        );
        Ok(())
    }

    /// Apply tenant where-clause to a delete statement.
    pub fn apply_delete_scope(
        &self,
        table: &str,
        stmt: &mut DeleteStatement,
    ) -> Result<(), TenantColumnError> {
        let tenant_column = self.tenant_column_for_table(table)?;
        stmt.and_where(
            Expr::col((Alias::new(table), Alias::new(tenant_column))).eq(self.tenant.as_str()),
        );
        Ok(())
    }

    /// Guard to ensure data being updated belongs to current tenant.
    pub fn ensure_tenant_match(&self, found_tenant: Option<&str>) -> Result<(), TenantColumnError> {
        match found_tenant {
            Some(found) if found == self.tenant.as_str() => Ok(()),
            Some(found) => Err(TenantColumnError::TenantMismatch {
                expected: self.tenant.as_str().to_string(),
                found: found.to_string(),
            }),
            None => Err(TenantColumnError::MissingTenant),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use sea_query::{Query, SqliteQueryBuilder};

    #[tokio::test]
    async fn default_tenant_column_is_used() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("db connect");
        let scope = TenantScope::new(db, Tenant::new("t-1"));

        let column = scope
            .tenant_column_for_table("users")
            .expect("column should exist");
        assert_eq!(column, "tenant_id");
    }

    #[tokio::test]
    async fn custom_resolver_overrides_column_per_table() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("db connect");
        let resolver = StaticTenantColumnResolver::new()
            .with_default_column("tenant_id")
            .with_table_column("users", "org_id");

        let scope = TenantScope::with_resolver(db, Tenant::new("t-1"), Arc::new(resolver));

        assert_eq!(
            scope.tenant_column_for_table("users").unwrap_or(""),
            "org_id"
        );
        assert_eq!(
            scope.tenant_column_for_table("orders").unwrap_or(""),
            "tenant_id"
        );
    }

    #[tokio::test]
    async fn strict_resolver_errors_for_missing_table_mapping() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("db connect");
        let resolver = StaticTenantColumnResolver::new().with_table_column("users", "org_id");
        let scope = TenantScope::with_resolver(db, Tenant::new("t-1"), Arc::new(resolver));

        let err = scope
            .tenant_column_for_table("orders")
            .expect_err("should fail when no mapping exists");

        assert_eq!(
            err,
            TenantColumnError::MissingTenantColumn {
                table: "orders".to_string()
            }
        );
    }

    #[tokio::test]
    async fn select_scope_injects_tenant_filter() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("db connect");
        let scope = TenantScope::new(db, Tenant::new("tenant-42"));

        let mut stmt = Query::select();
        stmt.from(Alias::new("users"));
        stmt.expr(Expr::col(Alias::new("id")));

        scope
            .apply_select_scope("users", &mut stmt)
            .expect("scope should apply");

        let sql = stmt.to_string(SqliteQueryBuilder);
        assert!(sql.contains("tenant_id"));
        assert!(sql.contains("tenant-42"));
    }

    #[tokio::test]
    async fn tenant_match_guard_detects_mismatch() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("db connect");
        let scope = TenantScope::new(db, Tenant::new("tenant-1"));

        let ok = scope.ensure_tenant_match(Some("tenant-1"));
        assert!(ok.is_ok());

        let mismatch = scope.ensure_tenant_match(Some("tenant-2"));
        assert!(matches!(
            mismatch,
            Err(TenantColumnError::TenantMismatch { .. })
        ));

        let missing = scope.ensure_tenant_match(None);
        assert!(matches!(missing, Err(TenantColumnError::MissingTenant)));
    }
}
