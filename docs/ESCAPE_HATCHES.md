# Escape Hatches — Using Raw Axum/Tower

This document shows how to implement the same behavior provided by the `#[get/post/...]` macros using raw `axum` and `tower` primitives. Use these escape hatches when you need full control, want to avoid macros, or need to interoperate with other Tower services.

Minimal example — manual route registration and server startup:

```rust
use axum::{Router, routing::get, Json};
use serde_json::json;
use std::net::SocketAddr;
use tokio::signal;

async fn hello() -> Json<serde_json::Value> {
    Json(json!({"message": "hello from raw axum"}))
}

#[tokio::main]
async fn main() {
    // Build router manually — this is what the macros generate under the hood
    let app = Router::new().route("/hello", get(hello));

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();

    // Graceful shutdown using the same strategy Nova uses internally
    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    let graceful = server.with_graceful_shutdown(async {
        let _ = signal::ctrl_c().await;
    });

    println!("listening on {}", addr);
    if let Err(err) = graceful.await {
        eprintln!("server error: {}", err);
    }
}
```

Notes and mapping to Nova macros:

- `#[get("/path")] fn handler(...)` → `Router::new().route("/path", get(handler))`
- Macro route registration uses `inventory` to submit `NovaRoute` instances at compile time; manual registration uses `Router::route(...)` at runtime.
- The macros generate `axum::routing::MethodRouter` values; use the same `get/post/put/patch` helpers from `axum::routing` for parity.
- To replicate middleware layers or state injection, wrap routers with `tower` layers or call `Router::with_state`.

When to use escape hatches:

- You need a route that is conditionally registered at runtime.
- You want to compose raw `tower::Service` components or plug into a non-Nova host.
- You need to avoid procedural macro-generated code (for example, in CI or tooling that inspects source without expanding macros).

Security and observability parity:

- To preserve the Nova observability layering (tracing, request ids, OpenAPI hooks), reuse the same middleware layers or call into the same plugin API if available. Plugins typically expose helpers that accept a `Router` and return an extended `Router`.

Conclusion

Everything the route macros do is expressible using raw `axum` and `tower` primitives. Use the escape hatch pattern above when you need more control or to debug macro-generated behavior.
