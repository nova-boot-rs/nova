# Nova Architecture

This document describes the current structure of the Nova framework and how a request moves through the system.

## Modules

### `nova-core`

This crate contains the runtime primitives used by applications:

- `NovaApp` for bootstrapping and serving the Axum router
- `NovaPlugin` for app-level extensions
- `NovaError` and `NovaResult` for standardized failures
- `ApiResponse`, `ListResponse`, and `EmptyResponse` for JSON output

### `nova-macros`

This crate provides attribute macros such as `#[get]` and `#[post]`.

The macros register routes at compile time by submitting route metadata into the inventory registry.

### `nova-sql`

This crate provides the database plugin used by the demo application.

It is responsible for:

- opening a SeaORM database connection
- exposing the connection to handlers through Axum state or extensions
- syncing entities when the plugin is configured to do so

## Request Flow

1. A handler is annotated with a route macro
2. The macro registers metadata for that route
3. `NovaApp` starts up and initializes plugins
4. The app reads registered routes from inventory
5. Axum serves the merged router
6. A handler returns either a structured success response or a `NovaError`
7. Axum converts the result into JSON plus the correct HTTP status code

## Response Contract

Nova keeps responses predictable:

- `ApiResponse<T>` is used for successful JSON payloads
- `NovaError` is used for failures
- `NovaError` maps to an HTTP status code automatically
- `ApiResponse::with_status` lets a handler choose a success status such as `201 Created`

## Why This Layout

The project is split into separate crates so the framework can evolve without forcing every application to depend on every feature.

- applications can use `nova-core` alone for routing and errors
- database support stays optional
- macros stay isolated from runtime code

## Example Application

The demo app in `example/demo` shows the current recommended style:

- route handlers return `NovaResult<Json<ApiResponse<T>>>`
- validation failures return `NovaError::ValidationError` or `NovaError::Custom`
- database access is injected through `Extension(DatabaseConnection)`
