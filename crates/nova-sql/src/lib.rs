use nova_core::{NovaPlugin, async_trait, axum::Extension, axum::Router};
pub use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Schema, Statement,
};
pub use sea_orm_migration::prelude::*;

pub struct NovaSql {
    pub db: DatabaseConnection,
    sync_tasks: Vec<
        Box<
            dyn for<'a> Fn(
                    &'a NovaSql,
                )
                    -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>
                + Send
                + Sync,
        >,
    >,
}

impl NovaSql {
    pub async fn connect(url: &str) -> Self {
        let db = Database::connect(url)
            .await
            .expect("Failed to connect to the database");
        Self {
            db,
            sync_tasks: Vec::new(),
        }
    }

    pub fn add_entity<E>(mut self) -> Self
    where
        E: EntityTrait + 'static,
    {
        self.sync_tasks
            .push(Box::new(|sql| Box::pin(sql.sync_entity::<E>())));
        self
    }

    pub async fn sync_entity<E>(&self)
    where
        E: EntityTrait,
    {
        let entity = E::default();
        let table_name = entity.table_name().to_string();
        let builder = self.db.get_database_backend();
        let schema = Schema::new(builder);

        // 1. Get the current columns in the database
        let existing_columns = self.get_table_columns(&table_name).await;

        if existing_columns.is_empty() {
            // Table doesn't exist, create it from scratch
            let stmt = builder.build(&schema.create_table_from_entity(entity.clone()));
            self.db.execute(stmt).await.expect("Failed to create table");
            println!("✅ Created table: {}", table_name);
        } else {
            // Table exists, check for missing columns (Evolution)
            let table_create_stmt = schema.create_table_from_entity(entity.clone());

            for column in table_create_stmt.get_columns() {
                let col_name = column.get_column_name().to_string();
                if !existing_columns.contains(&col_name) {
                    println!(
                        "✨ Adding missing column '{}' to '{}'",
                        col_name, table_name
                    );

                    // Generate ALTER TABLE statement
                    let alter_stmt = builder.build(
                        &sea_query::Table::alter()
                            .table(sea_query::Alias::new(&table_name))
                            .add_column(column.clone())
                            .to_owned(),
                    );

                    self.db
                        .execute(alter_stmt)
                        .await
                        .expect("Failed to alter table");
                }
            }
        }
    }

    /// Helper to fetch column names based on the database type
    async fn get_table_columns(&self, table_name: &str) -> Vec<String> {
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

    pub async fn run_migrations<M>(&self)
    where
        M: MigratorTrait,
    {
        M::up(&self.db, None).await.expect("Failed migrations");
    }
}

#[async_trait]
impl NovaPlugin for NovaSql {
    fn name(&self) -> &'static str {
        "NovaSql (Relational Engine)"
    }

    async fn on_init(&self) {
        println!("🗄️ Initializing SQL Plugin...");
        for task in &self.sync_tasks {
            task(self).await;
        }
    }

    fn extend_router(&self, router: Router) -> Router {
        router.layer(Extension(self.db.clone()))
    }
}
