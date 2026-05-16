# **Nova Framework - Complete Roadmap**

> Build resilient, observable microservices in Rust with developer-friendly ergonomics.

**Status Legend:** `[DONE]` `[IN PROGRESS]` `[PLANNED]`

---

## **Phase 1: Core Foundation** ✅ COMPLETE

The public API surface and fundamental abstractions.

- [DONE] `nova-core` as the single public runtime surface
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

---

## **Phase 3: API Ergonomics** ✅ COMPLETE

Macros and tools that make writing endpoints feel natural.

- [DONE] Routing macros: `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`
- [DONE] Request validation with `#[validate]`
- [DONE] Response helpers: `Json<T>`, `Html`, `Stream`
- [DONE] OpenAPI/Swagger generation hooks
- [DONE] API versioning: header-based and path-based
- [DONE] Pagination helpers: `Paginated<T>`, `Cursor<T>`

---

## **Phase 4: Resilience and Data** 🔄 IN PROGRESS

Make services robust against failure and connect to storage.

### Resilience (COMPLETE)
- [DONE] Circuit breaker with half-open probing
- [DONE] Retry with exponential backoff + jitter
- [DONE] Bulkhead (connection isolation)
- [DONE] Rate limiting: token bucket, sliding window

### SQL Plugin (DONE)
- [DONE] SeaORM integration with connection pooling
- [DONE] Migration runner
- [DONE] Read/write splitting (primary + round-robin replicas)
- [DONE] Cache hooks (Redis-backed query caching) with integration tests
- [DONE] Example handlers demonstrating `ReadWritePool` usage
- [DONE] Multi-tenancy column resolver

### NoSQL Plugin (IN PROGRESS)
- [IN PROGRESS] MongoDB adapter (scaffold added; runtime client wiring pending)
- [DONE] Redis adapter (cache + primary)
- [DONE] Document mapping with serde
- [DONE] Index management

### GraphDB Plugin (PLANNED)
- [PLANNED] Neo4j/SurrealDB adapter
- [PLANNED] Cypher/GraphQL query builder
- [PLANNED] Traversal helpers
- [PLANNED] Graph-to-JSON serialization

### Messaging (PLANNED)
- [PLANNED] Kafka producer/consumer
- [PLANNED] RabbitMQ adapter
- [PLANNED] NATS pub/sub
- [PLANNED] Event envelope standard
- [PLANNED] Dead letter queue support

### Cross-Cutting Data (PLANNED)
- [PLANNED] CQRS command/query separation helpers
- [PLANNED] Event sourcing primitives
- [PLANNED] Saga pattern coordinator for cross-service transactions

---

## **Phase 5: Service Discovery and Communication** 🔵 PLANNED

Let services find and talk to each other dynamically.

### Discovery Abstraction
- [PLANNED] `Discovery` trait in `nova-core`
- [PLANNED] Consul plugin (`nova-discovery-consul`)
- [PLANNED] etcd plugin (`nova-discovery-etcd`)
- [PLANNED] DNS/Kubernetes plugin (`nova-discovery-dns`)
- [PLANNED] Static list plugin for dev/testing (`nova-discovery-static`)

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

---

## **Phase 6: API Gateway** 🔵 PLANNED

A unified entry point that leverages all previous phases.

### Core Gateway
- [PLANNED] Reverse proxy with discovery-aware routing
- [PLANNED] Path-based and header-based routing rules
- [PLANNED] Request/response transformation middleware
- [PLANNED] WebSocket proxying

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

AuthN and AuthZ integrated across the framework.

### Authentication
- [PLANNED] JWT middleware with JWKS support
- [PLANNED] OAuth2/OpenID Connect integration
- [PLANNED] API key validation
- [PLANNED] Session management
- [PLANNED] Custom auth provider trait

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

### CLI (`nova-cli`) (IN PROGRESS)
- [IN PROGRESS] `nova new <name>` — scaffold a service
- [IN PROGRESS] `nova generate entity <name>` — generate CRUD
- [IN PROGRESS] `nova generate handler <name>` — generate endpoint
- [IN PROGRESS] `nova dev` — hot-reload development server
- [PLANNED] `nova docker build` — optimized container builds
- [PLANNED] `nova deploy` — push to cloud platforms

### Testing (PLANNED)
- [PLANNED] `nova-test` crate with service mocking
- [PLANNED] Integration test harness
- [PLANNED] Fixture management
- [PLANNED] Chaos testing helpers
- [PLANNED] Request recording and replay

### Examples and Documentation
- [PLANNED] `examples/rest-api` — Full RESTful CRUD service
- [PLANNED] `examples/event-worker` — Event-driven worker
- [PLANNED] `examples/api-gateway` — Gateway with multiple backends
- [PLANNED] `examples/grpc-service` — gRPC microservice
- [PLANNED] `examples/saga-orchestrator` — Distributed transaction
- [PLANNED] Full tutorial series on nova.rs

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
[━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━]
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
| 4. Resilience & Data | 🔄 IN PROGRESS | ~85% |
| 5. Service Discovery | 🔵 PLANNED | 0% |
| 6. API Gateway | 🔵 PLANNED | 0% |
| 7. Security | 🔵 PLANNED | 0% |
| 8. Developer Experience | 🔄 IN PROGRESS | ~40% |
| 9. Delivery & Operations | 🔵 PLANNED | 0% |

---

```text
crates/
├── nova-core/                  # Phase 1
├── nova-macros/                # Phase 3
├── nova-middleware/            # Phase 4
├── nova-observability/         # Phase 2-3
├── nova-resilience-store/      # Phase 4
├── plugins/
│   ├── nova-sql/               # Phase 4
│   ├── nova-nosql/             # Phase 4
│   ├── nova-graphdb/           # Phase 4
│   └── nova-messaging/         # Phase 4
├── discovery/
│   ├── nova-discovery/         # Phase 5 (trait)
│   ├── nova-discovery-consul/  # Phase 5
│   ├── nova-discovery-etcd/    # Phase 5
│   └── nova-discovery-dns/     # Phase 5
├── nova-client/                # Phase 5
├── nova-gateway/               # Phase 6
├── nova-auth/                  # Phase 7
├── nova-cli/                   # Phase 8
└── nova-test/                  # Phase 8
```