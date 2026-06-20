# Nova Demo

Multi-tenant task management demo showcasing the Nova framework and `nova-boot-sql`.

## Architecture

```
handlers → services → repositories → ReadWritePool → database
```

- **Handlers** — HTTP endpoints using `#[get]`/`#[post]` macros and extractors like `NovaDb`.
- **Services** — business rules and DTO mapping.
- **Repositories** — direct DB access via `ReadWritePool` typed CRUD methods (`fetch_all`, `insert_one`, etc.).
- **Entities** — SeaORM entities registered with `NovaSql::add_entity` for auto schema sync.

## Quick start

```bash
cd example/demo
cargo run
```

Server starts on `http://localhost:8080`.

## Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/users` | List all users |
| `GET` | `/users-paged` | Paginated user listing |
| `POST` | `/users` | Create user (validated) |
| `GET` | `/db-status` | Database backend / connectivity |
| `GET` | `/tenants/{id}/tasks` | List tasks scoped to a tenant |
| `POST` | `/tenants/{id}/tasks` | Create a tenant-scoped task |

## Example requests

```bash
# Create a user
curl -s -X POST http://localhost:8080/users \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","email":"alice@example.com"}'

# List users
curl -s http://localhost:8080/users

# Create a tenant-scoped task
curl -s -X POST http://localhost:8080/tenants/acme/tasks \
  -H 'Content-Type: application/json' \
  -d '{"title":"Sample Task","payload":{"note":"demo"}}'

# List tasks for a tenant
curl -s http://localhost:8080/tenants/acme/tasks
```

## Key patterns demonstrated

- **Read/write splitting** — `ReadWritePool::fetch_all` routes reads to replicas; `insert_one` routes to the primary.
- **Tenant isolation** — tasks are scoped via `Column::TenantId.eq(tenant_id)` in repository queries.
- **Request validation** — DTOs implement `NovaValidate`; handlers call `validate_request`.
- **Unified responses** — `ApiResponse`, `ListResponse`, `PaginatedResponse` from `nova-boot-middleware`.
- **Hot-reloaded config** — `spawn_json_file_hot_reloader` watches `config/runtime.json`.
- **Auto schema sync** — entities registered with `add_entity` have tables created/altered on startup.
- **Error handling** — `NovaResult<T>` with automatic `IntoResponse` conversion.

## Files of interest

- `src/main.rs` — app bootstrap, plugin registration, hot-reload config.
- `src/handlers/` — HTTP endpoints.
- `src/repositories/` — database access via `ReadWritePool` entity queries.
- `src/entities/` — SeaORM entity definitions.
- `config/runtime.json` — hot-reloaded runtime configuration.
