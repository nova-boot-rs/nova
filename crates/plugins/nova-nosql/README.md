# `nova-nosql`

Purpose

- NoSQL document & key-value plugin: MongoDB and Redis adapters, document mapping, and caching helpers.

Quick start

```rust
use nova_nosql::NovaNoSql;
let nosql = NovaNoSql::in_memory(); // for tests/dev
NovaApp::new("svc", 8080, state).add_plugin(nosql).run().await;
```

Highlights

- In-memory adapter for tests.
- Redis-backed document cache and MongoDB primary adapter.

Docs & examples

- See `crates/plugins/nova-nosql/src` and `crates/plugins/nova-nosql/examples` for usage notes.

Run the simple example (requires Redis for full run):

```bash
# from repository root
cargo run -p nova-nosql --example simple
```
