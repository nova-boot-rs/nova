use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Represents a graph node with labels and arbitrary properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub labels: Vec<String>,
    pub properties: HashMap<String, JsonValue>,
}

/// Represents an edge between two nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub rel_type: String,
    pub properties: HashMap<String, JsonValue>,
}

/// A subgraph containing nodes and edges returned by traversals.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSubgraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Supported query types for the adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphQuery {
    Cypher(String),
    GraphQl(String),
}
