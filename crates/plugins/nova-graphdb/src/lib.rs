//! Nova graph database abstractions and store implementations.

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
