//! Nova SQL abstractions and relational persistence helpers.
//!
//! This crate provides an opinionated integration with `sea-orm` for Nova
//! applications. It offers:
//!
//! - Connection helpers and a `NovaSql` builder for common database options.
//! - A `ReadWritePool` with round-robin replica selection for read scaling.
//! - Schema synchronization helpers and migration utilities compatible with
//!   `sea-orm-migration`.
//! - An Axum extractor `NovaDb` to access the injected `ReadWritePool` from
//!   request handlers.
//!
//! Examples
//!
//! ```rust,ignore
//! use nova_sql::NovaSql;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let sql = NovaSql::connect("sqlite::memory:", false).await;
//!     let pool = sql.read_write_pool();
//!     // Use `pool.read_sync()` for queries and `pool.write()` for writes.
//!     Ok(())
//! }
//! ```

// Include example source files in the crate-level rustdoc so readers can
// view runnable examples directly in the generated documentation.
#![doc = concat!("\n\n# Example: simple\n\n```rust\n", include_str!("../examples/simple.rs"), "\n```\n")]
#![doc = concat!("\n\n# Example: migrations\n\n```rust\n", include_str!("../examples/migrations.rs"), "\n```\n")]
#![doc = concat!("\n\n# Example: caching\n\n```rust\n", include_str!("../examples/caching.rs"), "\n```\n")]

pub mod cache;
pub mod connection;
pub mod migration;
pub mod plugin;
pub mod pool;
mod tenant;
mod tenant_middleware;
mod tenant_resolvers;
mod tenant_scope;

pub use cache::*;
pub use connection::{NovaSql, PoolOptions};
pub use pool::ReadWritePool;
pub use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Schema, Statement,
};
pub use sea_orm_migration::prelude::*;
pub use tenant::*;
pub use tenant_middleware::*;
pub use tenant_resolvers::*;
pub use tenant_scope::*;

mod extractors;
pub use extractors::NovaDb;

#[cfg(test)]
mod tests;
