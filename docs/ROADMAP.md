# Nova Roadmap

This document turns the feature spec into a staged implementation plan.

Status legend: [DONE], [IN PROGRESS], [PLANNED]

Current stage: Phase 4 (early)

## Phase 1: Core foundation
- [DONE] Keep `nova-core` as the public runtime surface.
- [DONE] Add a small set of reusable traits for plugins, requests, responses, and errors.
- [DONE] Split configuration, observability, and routing concerns into separate modules.

## Phase 2: Platform basics
- [DONE] Layered configuration: env vars, config files, defaults.
- [DONE] Hot reload for config changes.
- [DONE] Request IDs, structured logging, and tracing spans.
- [DONE] Health checks.
- [DONE] Graceful shutdown hooks.

## Phase 3: API ergonomics
- [DONE] Routing macros for common HTTP verbs.
- [DONE] Request validation and response helpers.
- [DONE] OpenAPI generation hooks.
- [DONE] Versioning and pagination helpers.

## Phase 4: Resilience and data
- [DONE] Circuit breaker, retry, bulkhead, and rate limiting primitives.
- [IN PROGRESS] SQL plugin with pooling, migrations, and cache hooks.
- [PLANNED] NoSQL plugin for document/key-value database support.
- [PLANNED] GraphDB plugin for graph database queries and traversal patterns.
- [PLANNED] Message queue adapters and event helpers.

## Phase 5: Security and delivery
- [PLANNED] Auth middleware for JWT, OAuth2, API keys, and RBAC.
- [PLANNED] mTLS-ready service integration points.
- [IN PROGRESS] Dev CLI, test helpers, and container/deployment support.

## Immediate next step
Complete Phase 4 data adapters: start with `nova-nosql`, then `nova-graphdb`, then queue/event adapters.
