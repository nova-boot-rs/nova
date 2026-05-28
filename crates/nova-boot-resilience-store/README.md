# `nova-boot-resilience-store`

Purpose

- Abstractions and adapters for distributed, durable state used by resilience primitives (e.g., Redis-backed stores for rate limiting and circuit state).

Quick notes

- Provides traits and optional Redis-backed implementation behind a feature flag (`redis-store`).
- Designed to be swapped for testing with in-memory implementations.

Example

```rust
// enable feature: redis-store
// use nova_resilience_store::RedisStore::new(...)
```

Docs

- See crate docs for trait definitions and adapters.
