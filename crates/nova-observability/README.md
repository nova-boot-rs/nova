# `nova-observability`

Purpose

- Observability helpers: tracing initialization, request IDs, OpenAPI hooks, and Prometheus metrics integration.

Quick start

```rust
use nova_observability::ObservabilityPlugin;

let obs = ObservabilityPlugin::new("my-service");
NovaApp::new("svc", 8080, state).add_plugin(obs).run().await;
```

Highlights

- Request ID propagation (Tower/axum integration).
- OpenAPI registration hooks.
- Tracing and metrics helpers for common telemetry patterns.

Docs & examples

- See crate docs and `example/` for typical usage patterns.
