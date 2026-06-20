use super::tenant::{StaticTenantColumnResolver, Tenant, TenantColumnError, TenantColumnResolver};
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel,
    QueryTrait, Select,
};
use sea_query::{Alias, DeleteStatement, Expr, SelectStatement, UpdateStatement};
use std::sync::Arc;

/// A database connection coupled with a tenant identity for scoped queries.
///
/// `TenantScope` enforces **row-level multi-tenancy** by automatically injecting
/// tenant-filter predicates into every SELECT, UPDATE, and DELETE statement that
/// passes through it. When combined with [`ReadWritePool`](crate::ReadWritePool)
/// you get both read/write splitting and tenant isolation.
///
/// # Tenant column resolution
///
/// The column used for tenant filtering is resolved through a
/// [`TenantColumnResolver`]. By default every table uses the column `"tenant_id"`.
/// A [`StaticTenantColumnResolver`] can be configured with per-table overrides:
///
/// ```rust,ignore
/// let resolver = StaticTenantColumnResolver::new()
///     .with_default_column("tenant_id")
///     .with_table_column("organizations", "org_id");
///
/// let scope = TenantScope::with_resolver(db, tenant, Arc::new(resolver));
/// ```
#[derive(Clone)]
pub struct TenantScope {
    pub db: DatabaseConnection,
    pub tenant: Tenant,
    resolver: Arc<dyn TenantColumnResolver>,
}

impl TenantScope {
    /// Create a new scope backed by the given connection and tenant identity.
    ///
    /// The resolver defaults to [`StaticTenantColumnResolver`] with column
    /// `"tenant_id"` for all tables.
    pub fn new(db: DatabaseConnection, tenant: Tenant) -> Self {
        Self {
            db,
            tenant,
            resolver: Arc::new(StaticTenantColumnResolver::new().with_default_column("tenant_id")),
        }
    }

    /// Create a scope with a custom [`TenantColumnResolver`].
    ///
    /// Use this when different tables use different column names for the tenant
    /// identifier (e.g. `"org_id"` for the `organizations` table).
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

    /// Return the tenant ID string used for row-level filtering.
    pub fn tenant_id(&self) -> &str {
        self.tenant.as_str()
    }

    /// Look up the tenant column name for the given table.
    ///
    /// Returns [`TenantColumnError::MissingTenantColumn`] when no column is
    /// configured for that table (only possible with a strict resolver that has
    /// no default).
    pub fn tenant_column_for_table(&self, table: &str) -> Result<&str, TenantColumnError> {
        self.resolver.tenant_column_for_table(table).ok_or_else(|| {
            TenantColumnError::MissingTenantColumn {
                table: table.to_string(),
            }
        })
    }

    /// Inject a tenant `WHERE` clause into a [`SelectStatement`].
    ///
    /// The clause has the form `` `table`.`tenant_col` = "<tenant_id>" ``.
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

    /// Inject a tenant `WHERE` clause into an [`UpdateStatement`].
    ///
    /// The clause has the form `` `table`.`tenant_col` = "<tenant_id>" ``.
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

    /// Inject a tenant `WHERE` clause into a [`DeleteStatement`].
    ///
    /// The clause has the form `` `table`.`tenant_col` = "<tenant_id>" ``.
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

    // ------------------------------------------------------------------
    // Tenant-isolated CRUD helpers
    // ------------------------------------------------------------------

    /// Fetch all rows matching a [`Select`] query, automatically scoped to the
    /// current tenant.
    ///
    /// The tenant filter (`WHERE tenant_col = '<tenant_id>'`) is injected into
    /// the query before execution.
    ///
    /// # Errors
    ///
    /// Returns [`DbErr::Custom`] when the tenant column cannot be resolved for
    /// the entity's table.
    pub async fn fetch_all<E>(&self, mut select_query: Select<E>) -> Result<Vec<E::Model>, DbErr>
    where
        E: EntityTrait,
    {
        let entity = E::default();
        let table_name = entity.table_name();
        let query_builder = select_query.query();
        self.apply_select_scope(table_name, query_builder)
            .map_err(|e| DbErr::Custom(e.to_string()))?;
        select_query.all(&self.db).await
    }

    /// Fetch at most one row matching a [`Select`] query, scoped to the current
    /// tenant.
    ///
    /// Returns `Ok(None)` when no matching row exists for this tenant.
    ///
    /// # Errors
    ///
    /// Returns [`DbErr::Custom`] when the tenant column cannot be resolved for
    /// the entity's table.
    pub async fn fetch_one<E>(&self, mut select_query: Select<E>) -> Result<Option<E::Model>, DbErr>
    where
        E: EntityTrait,
    {
        let entity = E::default();
        let table_name = entity.table_name();
        let query_builder = select_query.query();
        self.apply_select_scope(table_name, query_builder)
            .map_err(|e| DbErr::Custom(e.to_string()))?;
        select_query.one(&self.db).await
    }

    /// Insert a new row into the database within the current tenant context.
    ///
    /// **Important:** the caller is responsible for setting the tenant column
    /// on the active model before calling this method. For example:
    ///
    /// ```rust,ignore
    /// scope.insert_one(task::ActiveModel {
    ///     title: Set("Important task".into()),
    ///     tenant_id: Set(scope.tenant_id().into()),  // required
    ///     ..Default::default()
    /// }).await?;
    /// ```
    pub async fn insert_one<A>(
        &self,
        active_model: A,
    ) -> Result<<A::Entity as EntityTrait>::Model, DbErr>
    where
        A: ActiveModelTrait + ActiveModelBehavior + Send,
        <A::Entity as EntityTrait>::Model: IntoActiveModel<A>,
    {
        active_model.insert(&self.db).await
    }

    /// Verify that a fetched row's tenant value matches this scope's tenant.
    ///
    /// Returns [`TenantColumnError::TenantMismatch`] when the values differ, or
    /// [`TenantColumnError::MissingTenant`] when `found_tenant` is `None`.
    ///
    /// Use this as a safety check after fetching a row by primary key when you
    /// cannot rely on the query having been scoped automatically.
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
