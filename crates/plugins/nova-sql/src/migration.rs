use crate::connection::NovaSql;
use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DbBackend, DbErr, EntityTrait, Schema, Statement};
use sea_orm_migration::prelude::MigratorTrait;
use std::time::Duration;

#[async_trait]
pub(crate) trait SyncTask: Send + Sync {
    /// Execute the synchronization task against the provided `NovaSql` instance.
    async fn run(&self, sql: &NovaSql);
}

struct EntitySyncTask<E: EntityTrait> {
    _marker: std::marker::PhantomData<E>,
}

#[async_trait]
impl<E: EntityTrait + 'static> SyncTask for EntitySyncTask<E> {
    async fn run(&self, sql: &NovaSql) {
        sql.sync_entity::<E>().await;
    }
}

impl NovaSql {
    /// Register an entity type for automatic schema synchronization.
    pub fn add_entity<E: EntityTrait + 'static>(mut self) -> Self {
        self.sync_tasks.push(Box::new(EntitySyncTask::<E> {
            _marker: std::marker::PhantomData,
        }));
        self
    }

    /// Synchronize a single entity's columns with the database schema.
    ///
    /// If `allow_drop` is enabled, columns present in the database but
    /// missing from the model may be dropped (use with caution).
    pub async fn sync_entity<E>(&self)
    where
        E: EntityTrait,
    {
        let entity = E::default();
        let table_name = entity.table_name().to_string();
        let builder = self.db.get_database_backend();
        let schema = Schema::new(builder);

        let existing_columns = self.get_table_columns(&table_name).await;

        if existing_columns.is_empty() {
            // ... (Keep your existing Create Table logic)
        } else {
            let table_create_stmt = schema.create_table_from_entity(entity);
            let model_columns: Vec<String> = table_create_stmt
                .get_columns()
                .iter()
                .map(|c| c.get_column_name().to_string())
                .collect();

            // 1. ADD missing columns (Code -> DB)
            for column in table_create_stmt.get_columns() {
                let col_name = column.get_column_name().to_string();
                if !existing_columns.contains(&col_name) {
                    let alter_stmt = builder.build(
                        &sea_query::Table::alter()
                            .table(sea_query::Alias::new(&table_name))
                            .add_column(column.clone())
                            .to_owned(),
                    );
                    self.db.execute(alter_stmt).await.ok();
                }
            }

            if self.allow_drop {
                for db_col in existing_columns {
                    if !model_columns.contains(&db_col) {
                        println!(
                            "🗑️ Dropping unused column '{}' from '{}'",
                            db_col, table_name
                        );

                        let drop_stmt = builder.build(
                            &sea_query::Table::alter()
                                .table(sea_query::Alias::new(&table_name))
                                .drop_column(sea_query::Alias::new(&db_col))
                                .to_owned(),
                        );

                        if let Err(e) = self.db.execute(drop_stmt).await {
                            println!("⚠️ Could not drop column {}: {}", db_col, e);
                        }
                    }
                }
            }
        }
    }

    /// Helper to fetch column names based on the database type
    pub async fn get_table_columns(&self, table_name: &str) -> Vec<String> {
        let mut columns = Vec::new();

        match self.db.get_database_backend() {
            DbBackend::Sqlite => {
                let sql = format!("PRAGMA table_info('{}')", table_name);
                let res = self
                    .db
                    .query_all(Statement::from_string(DbBackend::Sqlite, sql))
                    .await
                    .unwrap();
                for row in res {
                    let name: String = row.try_get("", "name").unwrap_or_default();
                    columns.push(name);
                }
            }
            DbBackend::Postgres => {
                let sql =
                    "SELECT column_name FROM information_schema.columns WHERE table_name = $1";
                let res = self
                    .db
                    .query_all(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        sql,
                        vec![table_name.into()],
                    ))
                    .await
                    .unwrap();
                for row in res {
                    let name: String = row.try_get("", "column_name").unwrap_or_default();
                    columns.push(name);
                }
            }
            _ => println!("⚠️ Database backend not supported for auto-sync yet."),
        }
        columns
    }

    /// Run migrations using the provided `MigratorTrait` implementation.
    /// Returns a `Result` with the underlying `DbErr` on failure.
    pub async fn run_migrations<M>(&self) -> Result<(), DbErr>
    where
        M: MigratorTrait,
    {
        M::up(&self.db, None).await
    }

    /// Run migrations with simple retry logic.
    ///
    /// `attempts` must be >= 1 otherwise an explicit `DbErr::Custom` is returned.
    pub async fn run_migrations_with_retry<M>(
        &self,
        attempts: usize,
        delay: Duration,
    ) -> Result<(), DbErr>
    where
        M: MigratorTrait,
    {
        if attempts == 0 {
            return Err(DbErr::Custom(
                "run_migrations_with_retry requires at least one attempt".to_string(),
            ));
        }

        let mut last_err = None;
        for _ in 0..attempts {
            match M::up(&self.db, None).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_err = Some(e);
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(last_err.expect("migration attempts failed but no error captured"))
    }
}

#[cfg(test)]
mod tests {
    use super::NovaSql;
    use sea_orm::DbErr;
    use sea_orm_migration::prelude::{MigrationTrait, MigratorTrait};
    use std::time::Duration;

    struct DummyMigrator;

    impl MigratorTrait for DummyMigrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            Vec::new()
        }
    }

    #[tokio::test]
    async fn run_migrations_with_retry_returns_error_for_zero_attempts() {
        let sql = NovaSql::connect("sqlite::memory:", false).await;

        let err = sql
            .run_migrations_with_retry::<DummyMigrator>(0, Duration::from_millis(1))
            .await
            .expect_err("expected an explicit error for zero attempts");

        match err {
            DbErr::Custom(message) => {
                assert!(message.contains("at least one attempt"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
