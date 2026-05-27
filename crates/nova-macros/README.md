# `nova-macros`

Purpose

- Procedural macros for routing, validation, and request/response ergonomics used across Nova.

Quick notes

- Provides `#[get]`, `#[post]`, `#[validate]`, and other attribute macros.
- Use macros to reduce boilerplate and register routes via inventory.

Example

```rust
use nova_macros::{get, validate};

#[get("/items")]
fn list_items() -> Json<Vec<Item>> { ... }
```

Docs & development

- This crate is a `proc-macro` and compiles as part of the workspace. See `crates/nova-macros/src` for implementations and tests.
