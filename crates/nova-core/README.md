# `nova-core`

Purpose

- Core runtime and public API for Nova: `NovaApp`, `NovaPlugin`, lifecycle hooks, and configuration.

Quick start

- Construct `NovaApp`, add plugins, and run:

```rust
use nova_core::prelude::*;

let state = AppState::new(...);
NovaApp::new("my-service", 8080, state)
    .add_plugin(MyPlugin::new())
    .run()
    .await;
```

Highlights

- `NovaPlugin` trait to extend router and lifecycle.
- `NovaState` extractor for typed app state.
- `NovaError` → `IntoResponse` for consistent HTTP errors.

Docs & examples

- API docs: `cargo doc --package nova-core --no-deps` (or browse generated docs).
- Examples: see top-level `example/` demo for usage patterns.
