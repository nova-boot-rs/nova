//! Nova SQL abstractions and relational persistence helpers.

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

#[cfg(test)]
mod tests;
