use crate::{error::GraphDbError, traits::GraphStore, types::*};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Convert a `GraphSubgraph` into JSON.
pub fn graph_to_json(subgraph: &GraphSubgraph) -> Result<JsonValue, GraphDbError> {
    serde_json::to_value(subgraph).map_err(|e| GraphDbError::Serialization(e.to_string()))
}

/// High-level graph database wrapper used by application handlers.
///
/// `NovaGraphDb` composes a backend `GraphStore` implementation and exposes a
/// small, ergonomic API for executing queries, upserting nodes/edges, and
/// traversing subgraphs. The struct is `Clone` and intended to be injected into
/// request extensions by the plugin.
#[derive(Clone)]
pub struct NovaGraphDb {
    store: Arc<dyn GraphStore>,
}

impl NovaGraphDb {
    /// Construct a new wrapper around the provided store adapter.
    pub fn new(store: Arc<dyn GraphStore>) -> Self {
        Self { store }
    }

    /// Create an in-memory instance useful for testing and examples.
    pub fn in_memory() -> Self {
        Self::new(Arc::new(crate::memory::InMemoryGraphStore::default()))
    }

    /// Create a Neo4j-backed wrapper (adapter lives in `neo4j.rs`).
    pub fn neo4j(
        uri: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(crate::neo4j::Neo4jGraphStore::new(
            uri, user, password,
        )))
    }

    /// Create a SurrealDB-backed wrapper.
    pub fn surreal(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(crate::surreal::SurrealGraphStore::new(
            endpoint, namespace, database,
        )))
    }

    /// Create a SurrealDB-backed wrapper with basic auth credentials.
    pub fn surreal_with_auth(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(crate::surreal::SurrealGraphStore::new_with_auth(
            endpoint, namespace, database, username, password,
        )))
    }

    /// Execute a `GraphQuery` against the backend.
    pub async fn execute(&self, query: GraphQuery) -> Result<JsonValue, GraphDbError> {
        self.store.execute(query).await
    }

    /// Upsert a node.
    pub async fn upsert_node(&self, node: GraphNode) -> Result<(), GraphDbError> {
        self.store.upsert_node(node).await
    }

    /// Upsert an edge.
    pub async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphDbError> {
        self.store.upsert_edge(edge).await
    }

    /// Traverse the graph and return JSON representing the subgraph.
    pub async fn traverse_json(
        &self,
        start: &str,
        max_depth: usize,
    ) -> Result<JsonValue, GraphDbError> {
        let graph = self.store.traverse(start, max_depth).await?;
        graph_to_json(&graph)
    }
}
