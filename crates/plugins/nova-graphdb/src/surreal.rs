use crate::{builders::sanitize_symbol, error::GraphDbError, traits::GraphStore, types::*};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::Mutex;

pub(crate) fn surreal_result_rows(json: &JsonValue) -> Vec<JsonValue> {
    json.as_array()
        .and_then(|stmts| stmts.first())
        .and_then(|stmt| stmt.get("result"))
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default()
}

fn parse_surreal_record_id(value: &JsonValue) -> Option<String> {
    if let Some(id) = value.as_str() {
        return id
            .split(':')
            .nth(1)
            .map(ToString::to_string)
            .or_else(|| Some(id.to_string()));
    }

    let obj = value.as_object()?;
    if let Some(inner_id) = obj.get("id") {
        return parse_surreal_record_id(inner_id);
    }

    obj.get("tb").and_then(JsonValue::as_str).and_then(|tb| {
        obj.get("id")
            .and_then(JsonValue::as_str)
            .map(|id| format!("{tb}:{id}"))
    })
}

pub(crate) fn surreal_value_to_node(value: &JsonValue) -> Option<GraphNode> {
    if let Some(id) = value.as_str() {
        let parsed_id = parse_surreal_record_id(value)?;
        return Some(GraphNode {
            id: parsed_id,
            labels: vec![id.split(':').next().unwrap_or("node").to_string()],
            properties: HashMap::new(),
        });
    }

    let obj = value.as_object()?;

    let raw_id = obj.get("id")?;
    let id = parse_surreal_record_id(raw_id)?;

    let labels = raw_id
        .as_object()
        .and_then(|m| m.get("tb"))
        .and_then(JsonValue::as_str)
        .map(|tb| vec![tb.to_string()])
        .unwrap_or_else(|| vec!["node".to_string()]);

    let mut properties = obj
        .get("properties")
        .and_then(JsonValue::as_object)
        .cloned()
        .map(|m| m.into_iter().collect::<HashMap<_, _>>())
        .unwrap_or_default();

    for (k, v) in obj {
        if k != "id" && k != "properties" && !k.starts_with('_') {
            properties.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }

    Some(GraphNode {
        id,
        labels,
        properties,
    })
}

pub struct SurrealGraphStore {
    pub endpoint: String,
    pub namespace: String,
    pub database: String,
    client: reqwest::Client,
    username: Option<String>,
    password: Option<String>,
    token: Mutex<Option<String>>,
}

impl SurrealGraphStore {
    pub fn new(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            namespace: namespace.into(),
            database: database.into(),
            client: reqwest::Client::new(),
            username: None,
            password: None,
            token: Mutex::new(None),
        }
    }

    pub fn new_with_auth(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            namespace: namespace.into(),
            database: database.into(),
            client: reqwest::Client::new(),
            username: Some(username.into()),
            password: Some(password.into()),
            token: Mutex::new(None),
        }
    }

    async fn auth_token(&self) -> Result<Option<String>, GraphDbError> {
        let Some(username) = &self.username else {
            return Ok(None);
        };
        let Some(password) = &self.password else {
            return Ok(None);
        };

        let mut guard = self.token.lock().await;
        if let Some(token) = guard.as_ref() {
            return Ok(Some(token.clone()));
        }

        let endpoint = format!("{}/signin", self.endpoint.trim_end_matches('/'));
        let payload = serde_json::json!({
            "user": username,
            "pass": password,
        });

        let resp = self
            .client
            .post(endpoint)
            .header("Accept", "application/json")
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
                "surrealdb signin http status {}: {}",
                status, json
            )));
        }

        let token = json
            .get("token")
            .and_then(JsonValue::as_str)
            .or_else(|| json.get("result").and_then(JsonValue::as_str))
            .or_else(|| {
                json.get("result")
                    .and_then(JsonValue::as_object)
                    .and_then(|obj| obj.get("token"))
                    .and_then(JsonValue::as_str)
            })
            .ok_or_else(|| {
                GraphDbError::Backend(format!("surrealdb signin response missing token: {}", json))
            })?
            .to_string();

        *guard = Some(token.clone());
        Ok(Some(token))
    }

    async fn run_sql(&self, sql: &str) -> Result<JsonValue, GraphDbError> {
        let endpoint = format!("{}/sql", self.endpoint.trim_end_matches('/'));

        let mut request = self
            .client
            .post(endpoint)
            .header("surreal-ns", &self.namespace)
            .header("surreal-db", &self.database)
            .header("Accept", "application/json");

        if let Some(token) = self.auth_token().await? {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let resp = request
            .body(sql.to_string())
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
                "surrealdb http status {}: {}",
                status, json
            )));
        }

        Ok(json)
    }
}

#[async_trait]
impl GraphStore for SurrealGraphStore {
    async fn execute(&self, query: GraphQuery) -> Result<JsonValue, GraphDbError> {
        let sql = match query {
            GraphQuery::Cypher(q) => q,
            GraphQuery::GraphQl(q) => q,
        };
        self.run_sql(&sql).await
    }

    async fn upsert_node(&self, node: GraphNode) -> Result<(), GraphDbError> {
        let table = node
            .labels
            .first()
            .map(|v| v.to_ascii_lowercase())
            .unwrap_or_else(|| "node".to_string());
        let properties = serde_json::to_string(&node.properties)
            .map_err(|e| GraphDbError::Serialization(e.to_string()))?;
        let sql = format!(
            "UPSERT {table}:{} SET id = '{}', properties = {};",
            node.id, node.id, properties
        );
        self.run_sql(&sql).await.map(|_| ())
    }

    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphDbError> {
        let rel = sanitize_symbol(&edge.rel_type).to_ascii_lowercase();
        let props = serde_json::to_string(&edge.properties)
            .map_err(|e| GraphDbError::Serialization(e.to_string()))?;
        let sql = format!(
            "RELATE node:{}->{rel}->node:{} SET id = '{}', properties = {};",
            edge.from, edge.to, edge.id, props
        );
        self.run_sql(&sql).await.map(|_| ())
    }

    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>, GraphDbError> {
        let sql = format!("SELECT * FROM node:{};", node_id);
        let json = self.run_sql(&sql).await?;
        let rows = surreal_result_rows(&json);
        let Some(first) = rows.first() else {
            return Ok(None);
        };

        Ok(surreal_value_to_node(first).or_else(|| {
            Some(GraphNode {
                id: node_id.to_string(),
                labels: vec!["node".to_string()],
                properties: HashMap::new(),
            })
        }))
    }

    async fn neighbors(&self, node_id: &str) -> Result<Vec<GraphNode>, GraphDbError> {
        let mut out = Vec::new();

        let sql = format!("SELECT ->?->node AS neighbors FROM node:{};", node_id);
        let json = self.run_sql(&sql).await?;
        let rows = surreal_result_rows(&json);

        for row in rows {
            if let Some(neighbors) = row.get("neighbors").and_then(JsonValue::as_array) {
                for item in neighbors {
                    if let Some(node) = surreal_value_to_node(item) {
                        out.push(node);
                    }
                }
                continue;
            }

            if let Some(node) = surreal_value_to_node(&row) {
                out.push(node);
            }
        }

        Ok(out)
    }

    async fn traverse(&self, start: &str, max_depth: usize) -> Result<GraphSubgraph, GraphDbError> {
        let mut visited = HashSet::new();
        let mut q = VecDeque::from([(start.to_string(), 0usize)]);
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut edge_ids = HashSet::new();

        while let Some((current, depth)) = q.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(node) = self.get_node(&current).await? {
                nodes.push(node);
            }
            if depth >= max_depth {
                continue;
            }

            for n in self.neighbors(&current).await? {
                let synthetic_edge_id = format!("{}->{}", current, n.id);
                if edge_ids.insert(synthetic_edge_id.clone()) {
                    edges.push(GraphEdge {
                        id: synthetic_edge_id,
                        from: current.clone(),
                        to: n.id.clone(),
                        rel_type: "RELATED".to_string(),
                        properties: HashMap::new(),
                    });
                }
                q.push_back((n.id, depth + 1));
            }
        }

        Ok(GraphSubgraph { nodes, edges })
    }
}