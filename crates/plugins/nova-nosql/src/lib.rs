//! Nova NoSQL abstractions and document-store implementations.

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
