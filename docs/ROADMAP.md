# Nova Roadmap

This document turns the feature spec into a staged implementation plan.

## Phase 1: Core foundation
- Keep `nova-core` as the public runtime surface.
- Add a small set of reusable traits for plugins, requests, responses, and errors.
- Split configuration, observability, and routing concerns into separate modules.

## Phase 2: Platform basics
- Layered configuration: env vars, config files, defaults.
- Hot reload for config changes.
- Request IDs, structured logging, and tracing spans.
- Health checks and graceful shutdown hooks.

## Phase 3: API ergonomics
- Routing macros for common HTTP verbs.
- Request validation and response helpers.
- OpenAPI generation hooks.
- Versioning and pagination helpers.

## Phase 4: Resilience and data
- Circuit breaker, retry, bulkhead, and rate limiting primitives.
- SQL plugin with pooling, migrations, and cache hooks.
- Message queue adapters and event helpers.

## Phase 5: Security and delivery
- Auth middleware for JWT, OAuth2, API keys, and RBAC.
- mTLS-ready service integration points.
- Dev CLI, test helpers, and container/deployment support.

## Immediate next step
Build routing macros and request/response derive macros on top of the runtime and observability layers.
