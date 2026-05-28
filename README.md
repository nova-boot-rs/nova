# Nova-boot

**A batteries-included microservice framework for Rust — built on Axum.**

Build resilient, observable, and scalable services with first-class plugins for databases, messaging, discovery, and resilience primitives. Nova reduces integration work so you can focus on business logic.

[![Crates.io](https://img.shields.io/crates/v/nova-boot)](https://crates.io/crates/nova-boot)
[![Docs](https://docs.rs/nova-boot/badge.svg)](https://docs.rs/nova-boot)
[![CI](https://github.com/nova-boot/nova/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/nova/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#license)
---

---
---

## 🚀 Quick Start

Add the crates you need to `Cargo.toml` (choose plugins you need):

```toml
[dependencies]
nova-boot = "0.2"
# add plugin crates as needed
nova-boot-sql = "0.2"          # optional: SQL support (SeaORM)
nova-boot-nosql = "0.2"        # optional: NoSQL adapters
nova-boot-messaging = "0.2"    # optional: Kafka/RabbitMQ/NATS
nova-boot-observability = "0.2" # optional: tracing, metrics, OpenAPI
```

Tip: prefer adding only the plugins you use to keep binary size small. Use workspace dependency overrides for local development.

## Quick summary

- Plugin-first architecture: bring your DB pools, brokers, and middleware as swappable plugins.
- Batteries included: SQL, NoSQL, Graph, Messaging, Observability, Resilience, and discovery plugins.
- Ergonomic APIs: routing macros, request extractors, and validation helpers.
- Production-ready primitives: circuit breakers, retries, distributed rate limiting, and DLQ support.

---

## Why Nova?

Nova is a batteries‑included application framework built on Axum that removes repetitive integration work so teams can ship production services faster. For the long-form rationale and a framework comparison.

---

## 🚀 Quick Start

Add the crates you need to `Cargo.toml`:

```toml
[dependencies]
nova-boot = "0.2"
nova-boot-sql = "0.2"
nova-boot-observability = "0.2"
```

Create a minimal service:

```rust
use nova_boot::prelude::*;
use nova_sql::NovaSql;
use nova_observability::ObservabilityPlugin;

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
# visit http://localhost:3000/hello
```

---

## Key features

- Plugin architecture: modular runtime with `NovaPlugin` for middleware and services.
- Storage: `nova-boot-sql`, `nova-boot-nosql`, `nova-boot-graphdb` (SeaORM, MongoDB, Neo4j, etc.).
- Messaging: `nova-boot-messaging` with Kafka / RabbitMQ / NATS + DLQ support.
- Resilience: circuit breaker, retries, bulkheads, distributed rate limiting.
- Observability: structured logging, tracing, Prometheus metrics, OpenAPI hooks.
- Developer ergonomics: `#[get|post]`, `#[validate]`, and semantic request extractors.
- Examples and in‑memory adapters for testing and local development.

---

## Crates (high level)

See the `crates/` folder for all workspace members. Notable crates:

- `nova-boot` — runtime, plugin trait, app lifecycle
- `nova-boot-macros` — routing & validation macros
- `nova-boot-sql`, `nova-boot-nosql`, `nova-boot-graphdb`, `nova-boot-messaging` — data & messaging plugins
- `nova-boot-observability` — tracing, metrics, OpenAPI
- `nova-boot-client` — discovery-aware HTTP client
- `nova-boot-test` — integration test harness (reprioritized)

---

## Crate Overview

| Crate | Purpose |
|-------|---------|
| `nova-boot` | Runtime, plugin trait, app lifecycle, configuration |
| `nova-boot-macros` | Routing, validation, and service macros |
| `nova-boot-observability` | Tracing, metrics, OpenAPI |
| `nova-boot-middleware` | Rate limiting, circuit breaker, retry |
| `nova-boot-resilience-store` | Distributed state for resilience (Redis, in‑memory) |
| `nova-boot-sql` | SeaORM integration with read/write splitting and caching |
| `nova-boot-nosql` | Document & key‑value stores (MongoDB, Redis) |
| `nova-boot-graphdb` | Graph databases (Neo4j, SurrealDB) |
| `nova-boot-messaging` | Kafka, RabbitMQ, NATS with DLQ support |
| `nova-boot-data-patterns` | CQRS, Event Sourcing, Saga |
| `nova-boot-discovery` | Service discovery trait and backends (Consul, etcd, DNS, static) |
| `nova-boot-client` | Smart HTTP client with discovery‑aware load balancing |
| `nova-boot-gateway` | API Gateway (planned) |
| `nova-boot-auth` | Authentication & authorization (planned) |


---

## Docs & Roadmap

- Full roadmap: [ROADMAP.md](ROADMAP.md)
- Examples: [example/](example/)
- Document: [docs.rs](https://docs.rs/nova-boot/0.1.1)
---

## Contributing

Contributions welcome: code, docs, examples, or issues. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## License

Licensed under either MIT or Apache-2.0 at your option. See `LICENSE` files.

---

Built with ❤️ for the Rust community. If Nova helps you, consider sponsoring the project.

## License

- MIT license ([LICENSE-MIT](LICENSE))

---
