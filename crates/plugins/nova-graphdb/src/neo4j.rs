use crate::{builders::sanitize_symbol, error::GraphDbError, traits::GraphStore, types::*};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::{HashSet, VecDeque};

pub struct Neo4jGraphStore {
    pub uri: String,
    pub user: String,
    pub password: String,
    pub database: String,
    client: reqwest::Client,
}

impl Neo4jGraphStore {
    pub fn new(
        uri: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            user: user.into(),
            password: password.into(),
            database: "neo4j".to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn run_cypher(
        &self,
        statement: &str,
        parameters: JsonValue,
    ) -> Result<JsonValue, GraphDbError> {
        let endpoint = format!(
            "{}/db/{}/tx/commit",
            self.uri.trim_end_matches('/'),
            self.database
        );

        let payload = serde_json::json!({
            "statements": [
                {
                    "statement": statement,
                    "parameters": parameters
                }
            ]
        });

        let resp = self
            .client
            .post(endpoint)
            .basic_auth(&self.user, Some(&self.password))
            .json(&payload)
            .send()
            .await
            .map_err(|e| GraphDbError::Backend(e.to_string()))?;

        let status = resp.status();
        let json: JsonValue = resp
            .json()
            .await
            .map_err(|e| GraphDbError::Serialization(e.to_string()))?;

        if !status.is_success() {
            return Err(GraphDbError::Backend(format!(
                "neo4j http status {}: {}",
                status, json
            )));
        }

        let errors = json
            .get("errors")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();

        if !errors.is_empty() {
            return Err(GraphDbError::Backend(format!(
                "neo4j query error: {errors:?}"
            )));
        }

        Ok(json)
    }

    fn first_row(response: &JsonValue) -> Option<JsonValue> {
        response
            .get("results")
            .and_then(JsonValue::as_array)
            .and_then(|results| results.first())
            .and_then(|result| result.get("data"))
            .and_then(JsonValue::as_array)
            .and_then(|data| data.first())
            .and_then(|entry| entry.get("row"))
            .and_then(JsonValue::as_array)
            .and_then(|row| row.first())
            .cloned()
    }
}

#[async_trait]
impl GraphStore for Neo4jGraphStore {
    async fn execute(&self, query: GraphQuery) -> Result<JsonValue, GraphDbError> {
        let cypher = match query {
            GraphQuery::Cypher(q) => q,
            GraphQuery::GraphQl(_) => {
                return Err(GraphDbError::InvalidInput(
                    "Neo4j adapter accepts Cypher queries only".to_string(),
                ));
            }
        };
        self.run_cypher(&cypher, serde_json::json!({})).await
    }

    async fn upsert_node(&self, node: GraphNode) -> Result<(), GraphDbError> {
        let labels = if node.labels.is_empty() {
            "Node".to_string()
        } else {
            node.labels
                .iter()
                .map(|l| sanitize_symbol(l))
                .collect::<Vec<_>>()
                .join(":")
        };

        let cypher = format!("MERGE (n:{labels} {{id: $id}}) SET n += $props RETURN n.id");
        let params = serde_json::json!({
            "id": node.id,
            "props": node.properties
        });
        self.run_cypher(&cypher, params).await.map(|_| ())
    }

    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphDbError> {
        let rel_type = sanitize_symbol(&edge.rel_type);
        let cypher = format!(
            "MATCH (a {{id: $from}}), (b {{id: $to}}) MERGE (a)-[r:{rel_type} {{id: $id}}]->(b) SET r += $props RETURN r.id"
        );
        let params = serde_json::json!({
            "id": edge.id,
            "from": edge.from,
            "to": edge.to,
            "props": edge.properties
        });
        self.run_cypher(&cypher, params).await.map(|_| ())
    }

    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>, GraphDbError> {
        let cypher =
            "MATCH (n {id: $id}) RETURN {id: n.id, labels: labels(n), properties: properties(n)}";
        let response = self
            .run_cypher(cypher, serde_json::json!({ "id": node_id }))
            .await?;

        match Self::first_row(&response) {
            Some(value) => serde_json::from_value::<GraphNode>(value)
                .map(Some)
                .map_err(|e| GraphDbError::Serialization(e.to_string())),
            None => Ok(None),
        }
    }

    async fn neighbors(&self, node_id: &str) -> Result<Vec<GraphNode>, GraphDbError> {
        let cypher = "MATCH (a {id: $id})-[]->(b) RETURN {id: b.id, labels: labels(b), properties: properties(b)}";
        let response = self
            .run_cypher(cypher, serde_json::json!({ "id": node_id }))
            .await?;

        let rows = response
            .get("results")
            .and_then(JsonValue::as_array)
            .and_then(|results| results.first())
            .and_then(|result| result.get("data"))
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();

        rows.into_iter()
            .filter_map(|entry| {
                entry
                    .get("row")
                    .and_then(JsonValue::as_array)
                    .and_then(|row| row.first())
                    .cloned()
            })
            .map(|value| {
                serde_json::from_value::<GraphNode>(value)
                    .map_err(|e| GraphDbError::Serialization(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()
    }

    async fn traverse(&self, start: &str, max_depth: usize) -> Result<GraphSubgraph, GraphDbError> {
        let mut visited = HashSet::new();
        let mut q = VecDeque::from([(start.to_string(), 0usize)]);
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        while let Some((current, depth)) = q.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if let Some(node) = self.get_node(&current).await? {
                nodes.push(node.clone());
            }

            if depth >= max_depth {
                continue;
            }

            let cypher = "MATCH (a {id: $id})-[r]->(b) RETURN {id: r.id, from: a.id, to: b.id, rel_type: type(r), properties: properties(r)}";
            let response = self
                .run_cypher(cypher, serde_json::json!({ "id": current }))
                .await?;

            let rows = response
                .get("results")
                .and_then(JsonValue::as_array)
                .and_then(|results| results.first())
                .and_then(|result| result.get("data"))
                .and_then(JsonValue::as_array)
                .cloned()
                .unwrap_or_default();

            for entry in rows {
                if let Some(value) = entry
                    .get("row")
                    .and_then(JsonValue::as_array)
                    .and_then(|row| row.first())
                    .cloned()
                {
                    let edge: GraphEdge = serde_json::from_value(value)
                        .map_err(|e| GraphDbError::Serialization(e.to_string()))?;
                    q.push_back((edge.to.clone(), depth + 1));
                    edges.push(edge);
                }
            }
        }

        Ok(GraphSubgraph { nodes, edges })
    }
}