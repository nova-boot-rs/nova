use crate::{error::GraphDbError, types::*};
use async_trait::async_trait;
use serde_json::Value as JsonValue;

#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn execute(&self, query: GraphQuery) -> Result<JsonValue, GraphDbError>;
    async fn upsert_node(&self, node: GraphNode) -> Result<(), GraphDbError>;
    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphDbError>;
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>, GraphDbError>;
    async fn neighbors(&self, node_id: &str) -> Result<Vec<GraphNode>, GraphDbError>;
    async fn traverse(&self, start: &str, max_depth: usize) -> Result<GraphSubgraph, GraphDbError>;
}