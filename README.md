# 🌟 Nova

Nova is a high-productivity microservice framework for Rust built on top of Axum.

It is designed around convention over configuration, with a split crate layout for core HTTP behavior, macro-based route registration, and database integration through SeaORM.

## Features

- Attribute-based route handlers with `#[get]`, `#[post]`, `#[put]`, `#[patch]`, and `#[delete]`
- Automatic route discovery through the inventory registry
- Plugin-based application bootstrapping
- Standardized JSON responses for success and errors
- Shared `NovaError` and `NovaResult` types for handler ergonomics
- SeaORM-powered SQL integration in the `nova-sql` crate
- Built-in request logging through Tower HTTP tracing middleware

## Project Structure

- `crates/nova-core` - HTTP app runtime, response types, and error handling
- `crates/nova-macros` - route and controller attribute macros
- `crates/nova-sql` - database plugin and entity syncing helpers
- `example/demo` - runnable example showing controllers, entities, and database access

## Quick Start

### 1. Run the demo

```bash
cargo run --manifest-path example/demo/Cargo.toml
```

### 2. Call the endpoints

```bash
curl http://localhost:8080/hello
curl -X POST http://localhost:8080/echo -H "Content-Type: application/json" -d '{"content":"Hello Nova!"}'
curl http://localhost:8080/users
curl http://localhost:8080/db-status
curl http://localhost:8080/health
```

### 3. Create your own handler

```rust
use nova_core::{ApiResponse, Json, get};

#[get("/ping")]
pub async fn ping() -> Json<ApiResponse<&'static str>> {
	Json(ApiResponse::ok("pong"))
}
```

## API Guide

### Success responses

Use `ApiResponse` for structured JSON output. When you need a specific HTTP status, use `ApiResponse::with_status`.

```rust
Ok(Json(ApiResponse::with_status(StatusCode::CREATED, payload)))
```

### Error responses

Return `NovaResult<T>` and map failures to `NovaError` variants.

```rust
return Err(NovaError::NotFound("user not found".to_string()));
```

For custom HTTP codes, use `NovaError::Custom`.

```rust
return Err(NovaError::Custom {
	status: StatusCode::UNPROCESSABLE_ENTITY,
	error: "ValidationError".to_string(),
	message: "Message cannot be empty".to_string(),
});
```

## Example Controller

The demo application in `example/demo` shows how to:

- define request and response structs
- register handlers with route macros
- return structured JSON responses
- surface database errors through `NovaError`

See `example/demo/src/controller/controller.rs` for the full example.

## Architecture

Nova is intentionally small at the core:

- `nova-core` owns the application runtime and response contracts
- `nova-macros` turns annotated functions into registered routes
- `nova-sql` provides optional persistence support as a plugin

The request flow is:

1. The app starts through `NovaApp`
2. Plugins are initialized
3. Route macros submit handlers into the inventory registry
4. The app collects routes and builds the Axum router
5. Handlers return `ApiResponse` or `NovaError`, which convert into JSON responses

More detail is available in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and [docs/ERROR_HANDLING.md](docs/ERROR_HANDLING.md).

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Error Handling](docs/ERROR_HANDLING.md)

## Prerequisites for Optional Plugins

Some optional plugins (for example the `nova-discovery-etcd` crate) depend on native tooling to build their dependencies.

- `protoc` (Protocol Buffers compiler): required by the `etcd-client` dependency when building the etcd discovery plugin. If `protoc` is not available you may see an error like "Could not find `protoc`" during `cargo build` or `cargo test`.

Install `protoc` on Debian/Ubuntu with:

```bash
sudo apt-get update && sudo apt-get install -y protobuf-compiler
```

Or download a release from https://github.com/protocolbuffers/protobuf/releases and set the `PROTOC` environment variable to the `protoc` binary path if you prefer a custom location.

In GitHub Actions, the CI workflow installs `protoc` automatically before the build and test steps, so the repository does not need to keep a checked-in `PROTOC` override. If you want the local wrapper script, set it outside the repo, for example:

```bash
export PROTOC=/absolute/path/to/nova/scripts/protoc-wrapper.sh
```

or put the same setting in your personal `~/.cargo/config.toml`.

If you don't need to build the etcd plugin locally, run tests excluding that crate:

```bash
cargo test -p nova-discovery-static -p nova-discovery-consul
```

## Notes

- The demo uses SQLite through SeaORM.
- The response wrapper keeps HTTP status internal so the JSON stays stable while the transport status still changes.
- The demo echo endpoint accepts `{ "content": "..." }` and also tolerates the older `message` field.
