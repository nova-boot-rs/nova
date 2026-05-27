# `nova-sql`

Purpose

- SQL plugin integrating SeaORM with connection pooling, migrations, read/write splitting, and caching hooks.

Quick start

```rust
use nova_sql::NovaSql;
let sql = NovaSql::connect("sqlite::memory:", false).await;
NovaApp::new("svc", 8080, state).add_plugin(sql).run().await;
```

Highlights

- Migration runner (sea-orm-migration).
- Read/write pool separation and caching integration (Redis).
- Tenant-aware query hooks.

Docs & examples

- See `crates/plugins/nova-sql/src` and `crates/plugins/nova-sql/examples` for migration and pooling examples.

Run the simple example:

```bash
# from repository root
cargo run -p nova-sql --example simple
```

More examples:

- Migrations: `cargo run -p nova-sql --example migrations`
- Tenant-aware server: `cargo run -p nova-sql --example tenant` (runs an HTTP server on :4000)
- Caching: `cargo run -p nova-sql --example caching`

