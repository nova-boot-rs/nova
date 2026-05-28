# **Nova Framework - Complete Roadmap**

> Build resilient, observable microservices in Rust with developer-friendly ergonomics.

**Status Legend:** `[DONE]` `[IN PROGRESS]` `[PLANNED]`

---

## **Phase 1: Core Foundation** ✅ COMPLETE

The public API surface and fundamental abstractions.

- [DONE] `nova-boot` as the single public runtime surface
- [DONE] Reusable traits: `Plugin`, `Request`, `Response`, `Error`
- [DONE] Module split: configuration, observability, routing

---

## **Phase 2: Platform Basics** ✅ COMPLETE

What every production service needs to run safely.

- [DONE] Layered configuration: env vars → files → defaults
- [DONE] Hot reload for config changes
- [DONE] Request IDs, structured logging, tracing spans
- [DONE] Health check endpoints (`/health`, `/ready`)
- [DONE] Graceful shutdown with drain signals
- [DONE] Optional built-in HTTPS via `run_tls` (feature‑gated)
- [DONE] Debug mode with file/line numbers in error responses

---

## **Phase 3: API Ergonomics** ✅ COMPLETE

Macros and tools that make writing endpoints feel natural.

- [DONE] Routing macros: `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`
- [DONE] Request validation with `#[validate]`
- [DONE] Response helpers: `Json<T>`, `Html`, `Stream`
- [DONE] OpenAPI/Swagger generation hooks
- [DONE] API versioning: header-based and path-based
- [DONE] Pagination helpers: `Paginated<T>`, `Cursor<T>`
- [DONE] Escape hatches — ensure all macro behaviors are achievable via raw Axum/Tower

---

## **Phase 4: Resilience and Data** ✅ COMPLETE

Make services robust against failure and connect to storage.

### Resilience (COMPLETE)
- [DONE] Circuit breaker with half-open probing
- [DONE] Retry with exponential backoff + jitter
- [DONE] Bulkhead (connection isolation)
- [DONE] Rate limiting: token bucket, sliding window
- [DONE] Redis Lua scripts for atomic distributed rate limiting

### SQL Plugin (DONE)
- [DONE] SeaORM integration with connection pooling
- [DONE] Migration runner
- [DONE] Read/write splitting (primary + round-robin replicas)
- [DONE] Cache hooks (Redis-backed query caching) with integration tests
- [DONE] Example handlers demonstrating `ReadWritePool` usage
- [DONE] Multi-tenancy column resolver

### NoSQL Plugin (DONE)
- [DONE] MongoDB adapter
- [DONE] Redis adapter (cache + primary)
- [DONE] Document mapping with serde
- [DONE] Index management

### GraphDB Plugin (DONE)
- [DONE] Neo4j/SurrealDB adapter
- [DONE] Cypher/GraphQL query builder
- [DONE] Traversal helpers
- [DONE] Graph-to-JSON serialization

### Messaging (DONE)
- [DONE] Kafka producer/consumer
- [DONE] RabbitMQ adapter
- [DONE] NATS pub/sub
- [DONE] Event envelope standard
- [DONE] Dead letter queue support

### Request Extractors (DONE)
- [DONE] `NovaState<S>` app-state extractor in `nova-boot`
- [DONE] Named plugin extractors for `NovaSql`, `NovaNoSql`, `NovaGraphDb`, and `NovaMessaging`
- [DONE] Demo updated to use semantic extractors instead of raw `Extension<T>` for plugin resources

### Cross-Cutting Data (DONE)
- [DONE] CQRS command/query separation helpers
- [DONE] Event sourcing primitives
- [DONE] Saga pattern coordinator for cross-service transactions

---

## **Phase 5: Service Discovery and Communication** 🔄 IN PROGRESS

Let services find and talk to each other dynamically.

### Discovery Abstraction
- [DONE] `Discovery` trait in `nova-boot`
- [DONE] Static list plugin for dev/testing (`nova-boot-discovery-static`)
- [DONE] Consul plugin (`nova-boot-discovery-consul`)
- [DONE] etcd plugin (`nova-boot-discovery-etcd`)
- [PLANNED] DNS/Kubernetes plugin (`nova-boot-discovery-dns`)

### Service Registration
- [PLANNED] Auto-registration on startup with `#[service(name = "user-api")]`
- [PLANNED] Health reporting to registry (periodic heartbeat)
- [PLANNED] Graceful deregistration on shutdown

### Smart HTTP Client
- [PLANNED] `NovaClient` with service-name routing
- [PLANNED] Client-side load balancing: round-robin, random, weighted, latency-aware
- [PLANNED] Automatic retry for connection failures
- [PLANNED] Circuit breaking at the client level
- [PLANNED] mTLS handshake via registry-issued certificates

### gRPC Integration
- [PLANNED] Tonic integration for gRPC services
- [PLANNED] gRPC client with discovery-aware channel
- [PLANNED] Protocol buffer code generation helpers

### Dependency Injection (NEW — inspired by Nexios)
- [PLANNED] `#[inject]` macro for request-scoped dependencies
- [PLANNED] DI container with singleton/scoped/transient lifetimes
- [PLANNED] Integration with Axum's `FromRequestParts` for seamless extraction

### Testing (PRIORITIZED)
- [PRIORITIZED] Move `nova-test` earlier in the roadmap (see Phase 5 recommendations)
- [PLANNED] `nova-test` harness: `#[nova::test]` that spins up a `NovaApp` with in‑memory plugins
- [PLANNED] Request builders and fixtures for integration tests
- [PLANNED] Snapshot testing helpers and request recording/replay

---

## **Phase 6: API Gateway** 🔵 PLANNED

A unified entry point that leverages all previous phases. Built on Tower and hyper for zero-cost performance.

### Core Gateway
- [PLANNED] Reverse proxy with discovery-aware routing
- [PLANNED] Path-based and header-based routing rules
- [PLANNED] Request/response transformation middleware
- [PLANNED] WebSocket proxying
- [PLANNED] First-class WebSocket handlers (NEW)
- [PLANNED] Design note: Build as `tower::Service` implementation, not custom proxy

### Gateway Resilience
- [PLANNED] Circuit breaker per upstream service
- [PLANNED] Timeout and retry per route
- [PLANNED] Rate limiting per route and per consumer
- [PLANNED] Request caching at the edge

### Gateway Observability
- [PLANNED] Centralized access logging
- [PLANNED] Distributed trace context injection
- [PLANNED] Metrics aggregation for all upstreams
- [PLANNED] Real-time traffic dashboard

### Gateway Configuration
```rust
let gateway = NovaGateway::new()
    .with_discovery::<Consul>("http://consul:8500")
    .route("/api/users/**", "user-service")
    .route("/api/orders/**", "order-service")
    .route("/ws/**", "websocket-service")
    .with_circuit_breaker(failure_rate: 0.5)
    .with_global_rate_limit(1000, Duration::from_secs(1))
    .with_auth(JwtAuth::from_jwks("https://auth.example.com/.well-known/jwks.json"))
    .build();

gateway.serve("0.0.0.0:8080").await;
```

---

## **Phase 7: Security** 🔵 PLANNED

AuthN and AuthZ integrated across the framework. Pluggable — no forced user model.

### Authentication
- [PLANNED] JWT middleware with JWKS support
- [PLANNED] OAuth2/OpenID Connect integration
- [PLANNED] API key validation
- [PLANNED] Session management (NEW)
- [PLANNED] CSRF protection (NEW)
- [PLANNED] Custom auth provider trait

### Priority Note
- **Priority:** draft an Auth RFC and prioritize core AuthN middleware and API (JWT/OAuth2/API keys) immediately after Phase 5 work completes. Macros (e.g. `#[require_role]`) can follow once middleware primitives are solid.

### Authorization
- [PLANNED] Role-Based Access Control (RBAC) macros
- [PLANNED] Attribute-Based Access Control (ABAC)
- [PLANNED] Policy definition DSL
- [PLANNED] `#[require_role("admin")]` macro

### Transport Security
- [PLANNED] mTLS between services via registry
- [PLANNED] Automatic certificate rotation
- [PLANNED] Secrets management: Vault, AWS Secrets Manager, env-based
- [PLANNED] Secure configuration injection

### Audit
- [PLANNED] Audit logging trait
- [PLANNED] `#[audit(action = "user.created")]` macro
- [PLANNED] Audit log storage backends
- [PLANNED] Compliance reporting hooks

---

## **Phase 8: Developer Experience** 🔄 IN PROGRESS

Tools that make building with Nova fast and enjoyable.

### CLI (`nova-boot-cli`) (PLANNED)
- [PLANNED] `nova new <name>` — scaffold a service
- [PLANNED] `nova new <name> --template rest-api|event-worker|gateway|grpc` — starter templates (NEW)
- [PLANNED] `nova generate entity <name>` — generate CRUD
- [PLANNED] `nova generate handler <name>` — generate endpoint
- [PLANNED] `nova routes` — list all registered routes (NEW)
- [PLANNED] `nova config` — show resolved configuration (NEW)
- [PLANNED] `nova doctor` — check system health and dependencies (NEW)
- [PLANNED] `nova dev` — hot-reload development server
- [PLANNED] `nova docker build` — optimized container builds
- [PLANNED] `nova deploy` — push to cloud platforms

### Background Jobs / Task Queue (`nova-boot-tasks`) (PLANNED)
- [PLANNED] `nova-boot-tasks` crate scaffolded; implementation and adapters still pending
- [PLANNED] In-memory queue implementation for development and tests
- [PLANNED] Redis-backed queue adapter for production (atomic push/pop, visibility timeout)
- [PLANNED] `#[background_job]` macro (proc-macro) for easy job definition
- [PLANNED] Retries, exponential backoff, and DLQ support
- [PLANNED] Scheduled/delayed jobs API (cron-like and TTL-based delays)
- [PLANNED] Metrics (tasks processed, failures, retries) + tracing spans
- [PLANNED] Integration examples: `example/event-worker` + `example/demo` usage
- [PLANNED] Comprehensive tests and `nova-test` integration
- [PLANNED] Docs and a short tutorial on job patterns

### Documentation (NEW — prioritized)
- [PLANNED] "Getting Started" tutorial (based on `example/demo/`)
- [PLANNED] API reference for `nova-boot` public types
- [PLANNED] "Why Nova?" comparison page vs. raw Axum
- [PLANNED] `examples/rest-api` — Full RESTful CRUD service
- [PLANNED] `examples/event-worker` — Event-driven worker
- [PLANNED] `examples/api-gateway` — Gateway with multiple backends
- [PLANNED] `examples/grpc-service` — gRPC microservice
- [PLANNED] `examples/saga-orchestrator` — Distributed transaction
- [PLANNED] Full tutorial series on nova.rs

### Code Quality (NEW)
- [PLANNED] Split monolithic `lib.rs` files into logical modules for all crates
- [PLANNED] Refactor for `lib.rs` ≤ 50 lines per crate (re-export hub pattern)
- [PLANNED] Each module ≤ 300 lines

---

## **Phase 9: Delivery and Operations** 🔵 PLANNED

Production readiness and deployment tooling.

### Containerization
- [PLANNED] Multi-stage Dockerfile generation
- [PLANNED] Distroless and Alpine variants
- [PLANNED] Docker Compose templates for local dev

### Kubernetes
- [PLANNED] Helm chart generation
- [PLANNED] Kubernetes manifests for service + gateway
- [PLANNED] Operator pattern for Nova-native deployment
- [PLANNED] Sidecar injection for mesh integration

### CI/CD
- [PLANNED] GitHub Actions workflow templates
- [PLANNED] GitLab CI templates
- [PLANNED] Canary deployment strategy
- [PLANNED] Blue-green deployment strategy

### Telemetry Dashboards
- [PLANNED] Grafana dashboard templates
- [PLANNED] Prometheus metrics exporters
- [PLANNED] Alerting rules for common patterns
- [PLANNED] SLO/SLI tracking

---

## **Visual Timeline**

```
Phase 1        Phase 2        Phase 3         Phase 4           Phase 5-6
[━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━]
Core          Platform      API             Resilience        Discovery
Foundation    Basics        Ergonomics      & Data            & Gateway
                                                                
Phase 7        Phase 8        Phase 9
[━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━]
Security      Developer      Delivery &
              Experience     Operations
```

---

## **Current Status Summary**

| Phase | Status | Progress |
|-------|--------|----------|
| 1. Core Foundation | ✅ DONE | 100% |
| 2. Platform Basics | ✅ DONE | 100% |
| 3. API Ergonomics | ✅ DONE | 100% |
| 4. Resilience & Data | ✅ DONE | 100% |
| 5. Service Discovery & Communication | 🔄 IN PROGRESS | 40% |
| 6. API Gateway | 🔵 PLANNED | 0% |
| 7. Security | 🔵 PLANNED | 0% |
| 8. Developer Experience | 🔄 IN PROGRESS | 10% |
| 9. Delivery & Operations | 🔵 PLANNED | 0% |

---

## **New Tasks Added (from Nexios Analysis)**

| Task | Phase | Inspired By |
|------|-------|-------------|
| Debug mode error responses | 2 | Lesson 4: Error Handling Is an Art |
| Redis Lua scripts for atomic rate limiting | 4 | Deep Dive: DistributedRateLimiter |
| Dependency injection (`#[inject]`) | 5 | Nexios's `Depend` decorator |
| Gateway built on Tower/hyper (design note) | 6 | Expert Recommendation: Lean into Tower |
| First-class WebSocket handlers | 6 | Nexios feature: WebSocket support |
| Session management | 7 | Nexios feature: Session management |
| CSRF protection | 7 | Nexios feature: CSRF protection |
| `nova routes` CLI command | 8 | Lesson 2: Routing debug visibility |
| `nova config` CLI command | 8 | Developer debugging tools |
| `nova doctor` CLI command | 8 | Developer debugging tools |
| Starter templates for `nova new` | 8 | Nexios example projects |
| "Getting Started" tutorial | 8 | Lesson 7: Documentation Is Harder Than Coding |
| API reference docs | 8 | Lesson 7 |
| "Why Nova?" comparison page | 8 | Community adoption |
| Escape hatches from macros | 3 | Expert Recommendation: Avoid Macro Lock-in |
| Monolithic `lib.rs` refactor | 8 | Code quality and maintainability |

---

## **Crate Structure (Updated)**

```
crates/
├── nova-boot/                  # Phase 1
├── nova-boot-macros/                # Phase 3
├── nova-boot-middleware/            # Phase 4
├── nova-boot-observability/         # Phase 2-3
├── nova-boot-resilience-store/      # Phase 4
├── nova-boot-data-patterns/         # Phase 4 (CQRS, Event Sourcing, Saga)
├── plugins/
│   ├── nova-boot-sql/               # Phase 4
│   ├── nova-boot-nosql/             # Phase 4
│   ├── nova-boot-graphdb/           # Phase 4
│   └── nova-boot-messaging/         # Phase 4
├── discovery/
│   ├── nova-boot-discovery-consul/  # Phase 5
│   ├── nova-boot-discovery-etcd/    # Phase 5
│   └── nova-boot-discovery-dns/     # Phase 5
├── nova-boot-client/                # Phase 5
├── nova-boot-gateway/               # Phase 6
├── nova-boot-auth/                  # Phase 7
├── nova-boot-test/                  # Phase 5 (reprioritized)
├── nova-boot-cli/                   # Phase 8
└── nova-boot-tasks/                 # Phase 8 (new: lightweight in-process task queue)
```