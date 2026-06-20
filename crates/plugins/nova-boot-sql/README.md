# nova-boot-sql

SQL database adapter for Nova, built on [SeaORM](https://www.sea-ql.org/SeaORM/).

## Features

- **Read/write splitting** — `ReadWritePool` routes queries to replicas (round-robin) and mutations to the primary.
- **High-level CRUD** — `fetch_all`, `fetch_one`, `insert_one`, `update_one`, `delete_one` with automatic connection routing.
- **Multi-tenancy** — `TenantScope` injects row-level tenant filters into every query.
- **Schema sync** — auto-create/alter tables from SeaORM entity definitions.
- **Query caching** — optional in-memory or Redis-backed cache layer.
- **Replica management** — add replica connections at runtime.
- **Migration support** — run sea-orm-migration migrators during plugin init.

## Quick start

```rust
use nova_boot_sql::NovaSql;

let sql = NovaSql::connect("postgres://localhost/mydb", false).await;

// Register entities for schema sync
sql.add_entity::<myapp::UserEntity>();

// Obtain a read/write pool
let pool = sql.read_write_pool();

// Inject into a Nova app
NovaApp::new("svc", 8080, state).add_plugin(sql).run().await;
```

## Read/write pool

```rust
use nova_boot_sql::ReadWritePool;
use sea_orm::{EntityTrait, Set};

// Reads are load-balanced across replicas
let users = pool.fetch_all(User::find()).await?;

// Mutations go to the primary
let model = pool
    .insert_one(user::ActiveModel {
        id: sea_orm::NotSet,
        name: Set("Alice".into()),
        ..Default::default()
    })
    .await?;

// Sync access when you need a raw connection
let conn = pool.read_sync();
```

## Multi-tenancy

`TenantScope` wraps a connection with a tenant identity and injects `WHERE tenant_col = '<id>'` into every query:

```rust
use nova_boot_sql::{TenantScope, Tenant};

let scope = TenantScope::new(db, Tenant::new("tenant-42"));
let tasks = scope.fetch_all(Task::find()).await?; // tenant filter applied automatically
```

## Replicas

Add replica connections dynamically:

```rust
sql.add_replica_url("postgres://replica1/mydb").await?;
sql.add_replica_url("postgres://replica2/mydb").await?;
```

Or pass an existing connection:

```rust
pool.add_replica_sync(other_conn);
```

## Examples

```bash
# Tenant-aware HTTP server on :4000
cargo run -p nova-boot-sql --example tenant

# Schema migrations
cargo run -p nova-boot-sql --example migrations

# Query caching
cargo run -p nova-boot-sql --example caching
```

## Crate layout

| Module | Contents |
|---|---|
| [`NovaSql`] | Connection builder, replica registration, plugin entry point |
| [`ReadWritePool`] | Round-robin read/write pool with typed CRUD helpers |
| [`TenantScope`] | Tenant-isolated query execution |
| [`NovaDb`] | Axum extractor for `ReadWritePool` |
| [`migration`] | Schema sync and sea-orm-migration runner |
| [`cache`] | Query cache store trait + in-memory / Redis implementations |

[`NovaSql`]: https://docs.rs/nova-boot-sql/latest/nova_boot_sql/struct.NovaSql.html
[`ReadWritePool`]: https://docs.rs/nova-boot-sql/latest/nova_boot_sql/struct.ReadWritePool.html
[`TenantScope`]: https://docs.rs/nova-boot-sql/latest/nova_boot_sql/struct.TenantScope.html
[`NovaDb`]: https://docs.rs/nova-boot-sql/latest/nova_boot_sql/struct.NovaDb.html
