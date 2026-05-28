# Nova Demo

This demo illustrates core features of the Nova framework and the `nova-boot-sql` plugin using a small, multi-tenant task example.

Overview

- Layers: handlers -> services -> repositories -> SeaORM entities.
- Request/response DTOs live in `src/dtos`.
- Entities used for schema sync live in `src/entities`.
- The demo shows: hot-reloaded runtime config, `NovaSql` plugin, `NovaDb` extractor, tenant-scoped queries via `TenantScope`, request validation via `NovaValidate`, and standard error handling via `NovaError` / `NovaResult`.

Files of interest

- `src/main.rs` — app bootstrap, hot-reload config, register entities with `NovaSql`.
- `src/handlers` — HTTP endpoints (uses `#[get]`/`#[post]` macros and extractors).
- `src/services` — business rules and DTO -> repo mapping.
- `src/repositories` — direct DB access using `ReadWritePool`, `TenantScope`, and `sea_query`.
- `src/entities` — SeaORM entities; registered with `NovaSql::add_entity(...)` so tables are auto-synced.
- `config/runtime.json` — runtime config file (hot-reloaded).

Quick start

1. Build checks:

```bash
cd example/demo
cargo check
```

2. Run the demo (starts HTTP server on port 8080):

```bash
cd example/demo
cargo run
```

3. Example requests

Create a user:

```bash
curl -s -X POST http://localhost:8080/users \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","email":"alice@example.com"}' | jq
```

List users:

```bash
curl -s http://localhost:8080/users | jq
```

Create a tenant-scoped task:

```bash
curl -s -X POST http://localhost:8080/tenants/acme/tasks \
  -H 'Content-Type: application/json' \
  -d '{"title":"Sample Task","payload":{"note":"demo"}}' | jq
```

List tenant tasks:

```bash
curl -s http://localhost:8080/tenants/acme/tasks | jq
```

Endpoints

- `GET /users` — list users
- `GET /users-paged` — paginated listing
- `POST /users` — create user (validated via `NovaValidate`)
- `GET /db-status` — shows DB backend / connectivity
- `GET /tenants/{tenant_id}/tasks` — list tasks for tenant
- `POST /tenants/{tenant_id}/tasks` — create task for tenant

Key framework features demonstrated

- Hot-reloaded config: `spawn_json_file_hot_reloader(...)` reads `config/runtime.json` and updates `AppState` at runtime.
- Plugin system: `NovaSql::connect(...).add_entity::<...>()` registers SeaORM entities and performs schema sync on startup.
- Extractors: `NovaDb` is provided by the SQL plugin; handlers simply accept `NovaDb(pool)` to use DB connections.
- Tenant scoping: `TenantScope` (from `nova-boot-sql`) injects tenant filtering into queries so repositories don't repeat tenant logic.
- Validation: DTOs implement `NovaValidate` and handlers call `validate_request(&payload)?` to return structured validation errors.
- Error handling: Use `NovaResult<T>` (alias `Result<T, NovaError>`) in handlers — `NovaError` implements `IntoResponse` so HTTP responses are consistent.
- Responses: `ApiResponse`, `ListResponse`, and `PaginatedResponse` from `nova-boot-middleware` standardize JSON envelopes.

Schema sync / migrations

- The demo relies on `NovaSql::add_entity::<entities::Model>()` to automatically create/alter tables on startup. For production you should use formal migrations; this demo uses auto-sync for clarity.

Extending the demo

- Add more entities and register them in `main.rs` via `add_entity`.
- Replace the inlined SQL in repositories with SeaORM active models for richer ORM features.
- Add integration tests that run a fresh SQLite DB and exercise the HTTP endpoints.

Troubleshooting

- If handlers fail to get `NovaDb`, ensure `main.rs` adds the `sql_plugin` via `NovaApp::add_plugin(...)`.
- If schema sync doesn't create tables, confirm the entity is registered with `add_entity`.

License

- This demo follows the workspace license in the repo root.
