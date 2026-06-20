# Nova

**A batteries-included microservice framework for Rust — built on Axum.**

Build resilient, observable, and scalable services with first-class plugins for databases, messaging, discovery, and resilience primitives.

[![Crates.io](https://img.shields.io/crates/v/nova-boot)](https://crates.io/crates/nova-boot)
[![Docs](https://docs.rs/nova-boot/badge.svg)](https://docs.rs/nova-boot)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](#license)

---

## Quick Start

Add the crates you need to `Cargo.toml`:

```toml
[dependencies]
nova-boot = "0.1"
nova-boot-sql = "0.1"           # SQL (SeaORM)
nova-boot-messaging = "0.1"     # NATS / Kafka / RabbitMQ
nova-boot-observability = "0.1" # tracing, metrics, OpenAPI
```

Create a minimal service:

```rust
use nova_boot::prelude::*;
use nova_boot_sql::NovaSql;

#[get("/hello")]
fn hello() -> &'static str { "Hello from Nova!" }

#[tokio::main]
async fn main() {
    let db = NovaSql::connect("sqlite::memory:", false).await;
    let state = AppState::new(db);

    NovaApp::new("hello-service", 8080, state)
        .add_plugin(ObservabilityPlugin::new("hello-service"))
        .run()
        .await;
}
```

Run:

```bash
cargo run --bin hello-service
# visit http://localhost:8080/hello
```

---

## Why Nova?

Nova removes repetitive integration work so teams can ship production services faster. Plugin architecture means you bring only what you need — DB pools, brokers, middleware — as swappable plugins.

- Ergonomic APIs: routing macros (`#[get]`, `#[post]`), request extractors, validation helpers.
- Production-ready primitives: circuit breakers, retries, distributed rate limiting, DLQ support.
- Examples and in-memory adapters for testing and local development.

---

## Key Features

| Area | Crates |
|---|---|
| **Runtime** | `nova-boot` — app lifecycle, plugin trait, config, error types |
| **Macros** | `nova-boot-macros` — `#[get|post|put|patch|delete]`, `#[rest_controller]`, derives |
| **SQL** | `nova-boot-sql` — SeaORM with read/write splitting, multi-tenancy, schema sync, query caching |
| **NoSQL** | `nova-boot-nosql` — MongoDB and Redis document adapters with cache layer |
| **Graph** | `nova-boot-graphdb` — Neo4j/SurrealDB adapters, Cypher/GraphQL query builders |
| **Messaging** | `nova-boot-messaging` — NATS (async-nats), Kafka/RabbitMQ adapters, event envelope, DLQ |
| **Resilience** | `nova-boot-middleware` — circuit breaker, rate limiter, bulkhead |
| | `nova-boot-resilience-store` — distributed state (Redis / in-memory) |
| **Observability** | `nova-boot-observability` — tracing, request IDs, OpenAPI hooks |
| **Discovery** | `nova-boot-discovery-static` — static config |
| | `nova-boot-discovery-consul` — Consul backend |
| | `nova-boot-discovery-etcd` — etcd backend |
| **Client** | `nova-boot-client` — discovery-aware HTTP client |
| **Data patterns** | `nova-boot-data-patterns` — CQRS, Event Sourcing, Saga |
| **Task management** | `nova-boot-tasks` — background task utilities |
| **Planned** | `nova-boot-gateway`, `nova-boot-auth`, `nova-boot-cli` |

---

## Crate Overview

| Crate | Purpose |
|---|---|
| `nova-boot` | Runtime, `NovaPlugin` trait, `NovaApp`, `NovaConfigBuilder`, `NovaError` |
| `nova-boot-macros` | Route macros, `#[rest_controller]`, `NovaRequest`/`NovaResponse` derives |
| `nova-boot-sql` | SeaORM integration with `ReadWritePool`, `TenantScope`, schema sync, query cache |
| `nova-boot-nosql` | Document & key-value stores (MongoDB, Redis) |
| `nova-boot-graphdb` | Graph databases (Neo4j, SurrealDB) with in-memory store |
| `nova-boot-messaging` | NATS pub/sub, Kafka/RabbitMQ scaffolds, event envelope, DLQ, in-memory broker |
| `nova-boot-middleware` | Rate limiting, circuit breaker, bulkhead, response helpers |
| `nova-boot-resilience-store` | `ResilienceStore` trait, `RedisStore`, in-memory store |
| `nova-boot-observability` | Tracing init, request IDs, OpenAPI hook registry |
| `nova-boot-data-patterns` | CQRS, Event Sourcing, Saga patterns |
| `nova-boot-discovery-static` | Static service discovery |
| `nova-boot-discovery-consul` | Consul-backed service discovery |
| `nova-boot-discovery-etcd` | etcd-backed service discovery |
| `nova-boot-client` | Discovery-aware HTTP client with load balancing |
| `nova-boot-tasks` | Background task management |
| `nova-boot-gateway` | API Gateway (planned) |
| `nova-boot-auth` | Authentication & authorization (planned) |
| `nova-boot-cli` | CLI tooling (planned) |

---

## Examples

```bash
cd example/demo
cargo run
# serves on http://localhost:8080
```

See [example/demo/README.md](example/demo/README.md) for a full walkthrough.

---

## Documentation

- Full roadmap: [ROADMAP.md](ROADMAP.md)
- API docs: [docs.rs/nova-boot](https://docs.rs/nova-boot)

---

## License

Licensed under MIT. See [LICENSE](LICENSE).
