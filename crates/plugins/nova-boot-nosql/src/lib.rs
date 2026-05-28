//! Nova NoSQL abstractions and document-store implementations.
//!
//! This crate provides a uniform document-store abstraction for Nova apps and
//! includes adapters for in-memory, Redis, and MongoDB backends. The
//! `NovaNoSql` wrapper exposes a typed API for `get`, `upsert`, `delete`, and
//! index management, and can be combined with an optional caching adapter for
//! faster reads.
//!
//! Example
//!
//! ```rust,ignore
//! use nova_nosql::NovaNoSql;
//! # tokio_test::block_on(async {
//! let nosql = NovaNoSql::redis_primary("redis://127.0.0.1:6379", "app").await.unwrap();
//! let val: Option<String> = nosql.get("users", "user-1").await.unwrap();
//! # });
//! ```

// Include example source files into the crate-level rustdoc so users can
// inspect runnable examples directly from the generated documentation.
#![doc = concat!("\n\n# Example: simple\n\n```rust\n", include_str!("../examples/simple.rs"), "\n```\n")]

pub mod error;
pub mod mapper;
pub mod memory;
pub mod mongo;
pub mod plugin;
pub mod redis;
pub mod traits;
pub mod types;
pub mod wrapper;

pub use error::NoSqlError;
pub use mapper::SerdeDocumentMapper;
pub use memory::InMemoryDocumentStore;
pub use mongo::MongoDocumentStore;
pub use redis::RedisDocumentStore;
pub use traits::{DocumentCacheStore, DocumentStore};
pub use types::NoSqlIndex;
pub use wrapper::NovaNoSql;

#[cfg(test)]
mod tests;

mod extractors;
pub use extractors::NovaDocs;
