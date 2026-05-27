//! Graph database abstractions and store implementations for Nova.
//!
//! This crate provides a small adapter layer over different graph database
//! backends (Neo4j, Surreal, in-memory). It exposes a `NovaGraphDb` wrapper
//! that application handlers can extract to execute queries, upsert nodes and
//! edges, and traverse subgraphs. The crate also supplies lightweight
//! builders and types used across adapters.

// Embed example source in crate docs so users can view runnable examples
// directly in rustdoc.
#![doc = concat!("\n\n# Example: simple\n\n```rust\n", include_str!("../examples/simple.rs"), "\n```\n")]

pub mod builders;
pub mod error;
pub mod memory;
pub mod neo4j;
pub mod plugin;
pub mod surreal;
pub mod traits;
pub mod types;
pub mod wrapper;

pub use builders::{CypherQueryBuilder, GraphQlQueryBuilder};
pub use error::GraphDbError;
pub use memory::InMemoryGraphStore;
pub use neo4j::Neo4jGraphStore;
pub use surreal::SurrealGraphStore;
pub use traits::GraphStore;
pub use types::{GraphEdge, GraphNode, GraphQuery, GraphSubgraph};
pub use wrapper::{NovaGraphDb, graph_to_json};

#[cfg(test)]
mod tests;

mod extractors;
pub use extractors::NovaGraph;
