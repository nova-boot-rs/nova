use crate::connection::NovaSql;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, DbErr, DeleteResult, EntityTrait,
    IntoActiveModel, Select,
};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicUsize, Ordering},
};

/// Thread-safe read/write pool with round-robin replica selection for reads.
///
/// `ReadWritePool` wraps a primary [`DatabaseConnection`] together with a set of
/// read replicas. All write operations are automatically routed to the primary,
/// while read operations are load-balanced across replicas in round-robin order.
/// When no replicas are registered the primary connection is used for reads as a
/// graceful fallback.
///
/// The pool is cheaply cloneable — every clone shares the same internal
/// [`Arc`]-wrapped replica list — making it suitable for injection into Axum
/// handlers via [`crate::NovaDb`] or directly as [`Extension`](axum::Extension).
///
/// # Read/write splitting
///
/// ```rust,ignore
/// let pool: ReadWritePool = sql.read_write_pool();
///
/// // Writes go to the primary
/// pool.insert_one(my_model).await?;
///
/// // Reads are load-balanced across replicas
/// let results = pool.fetch_all(MyEntity::find()).await?;
/// ```
#[derive(Clone)]
pub struct ReadWritePool {
    primary: DatabaseConnection,
    replicas: Arc<RwLock<Vec<DatabaseConnection>>>,
    rr: Arc<AtomicUsize>,
}

impl ReadWritePool {
    /// Create a new pool from a primary connection and a set of replicas.
    ///
    /// `replicas` is an atomically reference-counted, synchronized list.  Pass
    /// [`Arc::new(RwLock::new(Vec::new()))`] to start with an empty replica pool
    /// and add replicas later with [`add_replica_sync`](Self::add_replica_sync).
    pub fn new(
        primary: DatabaseConnection,
        replicas: Arc<RwLock<Vec<DatabaseConnection>>>,
    ) -> Self {
        Self {
            primary,
            replicas,
            rr: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Select a replica connection for read queries.
    ///
    /// Replicas are chosen in round-robin order.  When the replica list is empty
    /// the primary connection is returned as a safe default.
    pub fn read_sync(&self) -> DatabaseConnection {
        let reps = self.replicas.read().expect("lock poisoned");
        if reps.is_empty() {
            return self.primary.clone();
        }

        let idx = self.rr.fetch_add(1, Ordering::Relaxed);
        reps[idx % reps.len()].clone()
    }

    /// Return the primary connection, which should be used for all write operations.
    pub fn write(&self) -> DatabaseConnection {
        self.primary.clone()
    }

    /// Add a replica connection to the pool.
    pub fn add_replica_sync(&self, conn: DatabaseConnection) {
        self.replicas.write().expect("lock poisoned").push(conn);
    }

    // ------------------------------------------------------------------
    // High-level CRUD helpers with automatic read/write splitting
    // ------------------------------------------------------------------

    /// Execute a [`Select`] query against a read replica.
    ///
    /// Returns all matching rows.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let users = pool.fetch_all(User::find()).await?;
    /// ```
    pub async fn fetch_all<E>(&self, select_query: Select<E>) -> Result<Vec<E::Model>, DbErr>
    where
        E: EntityTrait,
    {
        let conn = self.read_sync();
        select_query.all(&conn).await
    }

    /// Execute a [`Select`] query against a read replica, returning at most one row.
    ///
    /// Returns `Ok(None)` when no matching row exists.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(user) = pool.fetch_one(User::find_by_id(42)).await? {
    ///     // …
    /// }
    /// ```
    pub async fn fetch_one<E>(&self, select_query: Select<E>) -> Result<Option<E::Model>, DbErr>
    where
        E: EntityTrait,
    {
        let conn = self.read_sync();
        select_query.one(&conn).await
    }

    /// Insert a new row via an [`ActiveModel`](sea_orm::ActiveModelTrait).
    ///
    /// The insert is always executed against the primary database connection.
    ///
    /// # Type parameters
    ///
    /// * `A` — the active-model type (must implement
    ///   [`ActiveModelTrait`], [`ActiveModelBehavior`], and [`Send`]).
    /// * The entity's `Model` must be convertible into `A` via
    ///   [`IntoActiveModel`].
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let model = pool.insert_one(user::ActiveModel {
    ///     name: Set("Alice".into()),
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
        let conn = self.write();
        active_model.insert(&conn).await
    }

    /// Update an existing row via an [`ActiveModel`](sea_orm::ActiveModelTrait).
    ///
    /// The update is always executed against the primary database connection.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut user: user::ActiveModel = existing_user.into();
    /// user.name = Set("Bob".into());
    /// pool.update_one(user).await?;
    /// ```
    pub async fn update_one<A>(
        &self,
        active_model: A,
    ) -> Result<<A::Entity as EntityTrait>::Model, DbErr>
    where
        A: ActiveModelTrait + ActiveModelBehavior + Send,
        <A::Entity as EntityTrait>::Model: IntoActiveModel<A>,
    {
        let conn = self.write();
        active_model.update(&conn).await
    }

    /// Delete a row by its active model.
    ///
    /// The delete is always executed against the primary database connection.
    ///
    /// Returns a [`DeleteResult`] indicating how many rows were affected.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = pool.delete_one(to_delete).await?;
    /// assert_eq!(result.rows_affected, 1);
    /// ```
    pub async fn delete_one<A>(&self, active_model: A) -> Result<DeleteResult, DbErr>
    where
        A: ActiveModelTrait + ActiveModelBehavior + Send,
    {
        let conn = self.write();
        active_model.delete(&conn).await
    }
}

impl NovaSql {
    /// Build a [`ReadWritePool`] from this [`NovaSql`] instance.
    ///
    /// The returned pool shares the internal replica list via [`Arc`]; changes
    /// (e.g. new replicas added through the pool) are visible to the original
    /// `NovaSql` and vice versa.
    pub fn read_write_pool(&self) -> ReadWritePool {
        ReadWritePool::new(self.db.clone(), self.replicas.clone())
    }
}
