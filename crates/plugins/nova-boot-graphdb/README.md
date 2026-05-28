# `nova-boot-graphdb`

Purpose

- Graph database plugin with adapters for Neo4j/SurrealDB and query helpers for traversals.

Quick start

```rust
use nova_graphdb::NovaGraphDb;
let graph = NovaGraphDb::in_memory();
NovaApp::new("svc", 8080, state).add_plugin(graph).run().await;
```

Highlights

- Query builders and graph-to-JSON helpers.
- In-memory adapter for local testing.

Docs & examples

- See `crates/plugins/nova-boot-graphdb/src` for API and examples.
