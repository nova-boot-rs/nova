use crate::{error::GraphDbError, traits::GraphStore, types::*};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct InMemoryGraphStore {
    nodes: Arc<RwLock<HashMap<String, GraphNode>>>,
    edges: Arc<RwLock<HashMap<String, GraphEdge>>>,
}

#[async_trait]
impl GraphStore for InMemoryGraphStore {
    async fn execute(&self, _query: GraphQuery) -> Result<JsonValue, GraphDbError> {
        Err(GraphDbError::NotImplemented(
            "in-memory store does not parse free-form query text",
        ))
    }

    async fn upsert_node(&self, node: GraphNode) -> Result<(), GraphDbError> {
        if node.id.is_empty() {
            return Err(GraphDbError::InvalidInput(
                "node id cannot be empty".to_string(),
            ));
        }
        self.nodes.write().await.insert(node.id.clone(), node);
        Ok(())
    }

    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphDbError> {
        if edge.id.is_empty() {
            return Err(GraphDbError::InvalidInput(
                "edge id cannot be empty".to_string(),
            ));
        }

        let nodes = self.nodes.read().await;
        if !nodes.contains_key(&edge.from) || !nodes.contains_key(&edge.to) {
            return Err(GraphDbError::InvalidInput(
                "edge endpoints must exist before edge upsert".to_string(),
            ));
        }
        drop(nodes);

        self.edges.write().await.insert(edge.id.clone(), edge);
        Ok(())
    }

    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>, GraphDbError> {
        Ok(self.nodes.read().await.get(node_id).cloned())
    }

    async fn neighbors(&self, node_id: &str) -> Result<Vec<GraphNode>, GraphDbError> {
        let edges = self.edges.read().await;
        let nodes = self.nodes.read().await;
        let mut out = Vec::new();

        for edge in edges.values() {
            if edge.from == node_id
                && let Some(node) = nodes.get(&edge.to)
            {
                out.push(node.clone());
            }
        }

        Ok(out)
    }

    async fn traverse(&self, start: &str, max_depth: usize) -> Result<GraphSubgraph, GraphDbError> {
        let nodes_map = self.nodes.read().await;
        if !nodes_map.contains_key(start) {
            return Ok(GraphSubgraph::default());
        }
        let edges_map = self.edges.read().await;

        let mut visited: HashSet<String> = HashSet::new();
        let mut q: VecDeque<(String, usize)> = VecDeque::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        visited.insert(start.to_string());
        q.push_back((start.to_string(), 0));

        while let Some((current, depth)) = q.pop_front() {
            if let Some(node) = nodes_map.get(&current) {
                nodes.push(node.clone());
            }

            if depth >= max_depth {
                continue;
            }

            for edge in edges_map.values() {
                if edge.from == current {
                    edges.push(edge.clone());

                    if !visited.contains(&edge.to) {
                        visited.insert(edge.to.clone());
                        q.push_back((edge.to.clone(), depth + 1));
                    }
                }
            }
        }

        Ok(GraphSubgraph { nodes, edges })
    }
}
