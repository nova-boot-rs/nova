## 🤔 Why Nova?

> *“Rust ecosystem currently does not have a one‑stop microservice framework equivalent to Spring Boot.”*

That’s why Nova exists. Other Rust web frameworks give you a fast HTTP layer; **Nova gives you the entire application platform** so you can spend your time on business logic, not on gluing libraries together.

| With Axum / Actix / Rocket | With Nova |
|-----------------------------|-----------|
| 2–4 weeks wiring up SeaORM, Redis, Kafka, retry, circuit breaker, tracing, OpenAPI, validation, service discovery… | **A few hours** adding plugins and writing handlers |
| Every service reinvents the same integration code | Consistent, reusable plugins across all your services |
| Manual error handling, boilerplate `IntoResponse` impls | Built‑in `NovaError` with automatic JSON responses |
| Hardcoded URLs, manual load balancing | Service‑name routing with `NovaClient` |
| No standard multi‑tenancy pattern | `TenantScope<T>` extractor, zero additional code |

**Nova is Spring Boot for Rust** — with the performance and safety of Axum underneath.

---

## 🧰 Features

- **Plugin architecture** – Compose database pools, message brokers, and middleware as swappable plugins.
- **Multi‑protocol data** – SQL (SeaORM), NoSQL (MongoDB, Redis), Graph (Neo4j, SurrealDB), and Messaging (Kafka, RabbitMQ, NATS) all with a unified interface.
- **Resilience built‑in** – Circuit breaker, retry, rate limiter, and bulkhead from day one, distributed via Redis.
- **Service Discovery** – Consul, etcd, DNS/K8s, or static lists; register and discover services without hardcoding URLs.
- **Smart HTTP Client** – Load‑balance across instances (round‑robin, random, latency‑aware) with automatic retry and circuit breaking.
- **Multi‑tenancy** – `TenantScope<T>` automatically scopes queries to the current tenant.
- **Advanced patterns** – CQRS, Event Sourcing, and Saga orchestration built into the framework.
- **Observability** – Request IDs, structured logging, OpenTelemetry traces, Prometheus metrics, and OpenAPI docs.
- **Macro‑driven ergonomics** – `#[get]`, `#[post]`, `#[validate]`, `#[require_role]` keep your handlers clean.
- **Dev‑CLI** – `nova new`, `nova generate`, `nova dev` accelerate development.

---
