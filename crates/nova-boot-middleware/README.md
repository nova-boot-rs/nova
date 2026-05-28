# `nova-boot-middleware`

Purpose

- Collection of common middleware implemented for Nova: rate limiting, circuit breakers, retries, and bulkheads.

Usage

- Middleware is normally applied by plugins or via `NovaPlugin::extend_router`:

```rust
router = router.layer(nova_middleware::rate_limit::layer(...));
```

Highlights

- Tower-compatible layers.
- Integrations with `nova-boot-resilience-store` for distributed state (Redis).

Docs

- See crate documentation for available layers and configuration examples.
