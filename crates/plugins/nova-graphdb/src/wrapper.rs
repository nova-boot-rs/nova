use crate::{error::GraphDbError, traits::GraphStore, types::*};
use serde_json::Value as JsonValue;
use std::sync::Arc;

pub fn graph_to_json(subgraph: &GraphSubgraph) -> Result<JsonValue, GraphDbError> {
    serde_json::to_value(subgraph).map_err(|e| GraphDbError::Serialization(e.to_string()))
}

#[derive(Clone)]
pub struct NovaGraphDb {
    store: Arc<dyn GraphStore>,
}

impl NovaGraphDb {
    pub fn new(store: Arc<dyn GraphStore>) -> Self {
        Self { store }
    }

    pub fn in_memory() -> Self {
        Self::new(Arc::new(crate::memory::InMemoryGraphStore::default()))
    }

    pub fn neo4j(
        uri: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(crate::neo4j::Neo4jGraphStore::new(
            uri, user, password,
        )))
    }

    pub fn surreal(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(crate::surreal::SurrealGraphStore::new(
            endpoint, namespace, database,
        )))
    }

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

    pub async fn execute(&self, query: GraphQuery) -> Result<JsonValue, GraphDbError> {
        self.store.execute(query).await
    }

    pub async fn upsert_node(&self, node: GraphNode) -> Result<(), GraphDbError> {
        self.store.upsert_node(node).await
    }

    pub async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphDbError> {
        self.store.upsert_edge(edge).await
    }

    pub async fn traverse_json(
        &self,
        start: &str,
        max_depth: usize,
    ) -> Result<JsonValue, GraphDbError> {
        let graph = self.store.traverse(start, max_depth).await?;
        graph_to_json(&graph)
    }
}
