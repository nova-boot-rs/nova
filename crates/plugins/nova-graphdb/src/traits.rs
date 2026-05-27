use crate::{error::GraphDbError, types::*};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

/// Adapter trait for graph database backends.
///
/// Implement this trait to support a new graph backend (Neo4j, Surreal, in-memory).
#[async_trait]
pub trait GraphStore: Send + Sync {
    /// Execute a query and return JSON results.
    async fn execute(&self, query: GraphQuery) -> Result<JsonValue, GraphDbError>;

    /// Upsert a node into the store.
    async fn upsert_node(&self, node: GraphNode) -> Result<(), GraphDbError>;

    /// Upsert an edge into the store.
    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphDbError>;

    /// Get a node by id.
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>, GraphDbError>;

    /// Return immediate neighbor nodes for `node_id`.
    async fn neighbors(&self, node_id: &str) -> Result<Vec<GraphNode>, GraphDbError>;

    /// Traverse from `start` and return a `GraphSubgraph` up to `max_depth`.
    async fn traverse(&self, start: &str, max_depth: usize) -> Result<GraphSubgraph, GraphDbError>;
}
