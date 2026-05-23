use async_trait::async_trait;
use nova_core::{NovaPlugin, async_trait as nova_async_trait, axum::Extension, axum::Router};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

fn sanitize_symbol(value: &str) -> String {
    let cleaned = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>();
    if cleaned.is_empty() {
        "Node".to_string()
    } else {
        cleaned
    }
}

fn surreal_result_rows(json: &JsonValue) -> Vec<JsonValue> {
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

fn surreal_value_to_node(value: &JsonValue) -> Option<GraphNode> {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub labels: Vec<String>,
    pub properties: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub rel_type: String,
    pub properties: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSubgraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphQuery {
    Cypher(String),
    GraphQl(String),
}

#[derive(Debug)]
pub enum GraphDbError {
    Backend(String),
    NotImplemented(&'static str),
    InvalidInput(String),
    Serialization(String),
}

impl fmt::Display for GraphDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(msg) => write!(f, "backend error: {msg}"),
            Self::NotImplemented(msg) => write!(f, "not implemented: {msg}"),
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::Serialization(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

impl std::error::Error for GraphDbError {}

#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn execute(&self, query: GraphQuery) -> Result<JsonValue, GraphDbError>;
    async fn upsert_node(&self, node: GraphNode) -> Result<(), GraphDbError>;
    async fn upsert_edge(&self, edge: GraphEdge) -> Result<(), GraphDbError>;
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>, GraphDbError>;
    async fn neighbors(&self, node_id: &str) -> Result<Vec<GraphNode>, GraphDbError>;
    async fn traverse(&self, start: &str, max_depth: usize) -> Result<GraphSubgraph, GraphDbError>;
}

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

pub struct CypherQueryBuilder {
    query: String,
}

impl CypherQueryBuilder {
    pub fn new() -> Self {
        Self {
            query: String::new(),
        }
    }

    pub fn raw(mut self, fragment: impl AsRef<str>) -> Self {
        if !self.query.is_empty() {
            self.query.push(' ');
        }
        self.query.push_str(fragment.as_ref());
        self
    }

    pub fn match_node(mut self, alias: &str, label: &str) -> Self {
        self.query.push_str(&format!("MATCH ({alias}:{label}) "));
        self
    }

    pub fn where_eq(mut self, alias: &str, field: &str, value: &str) -> Self {
        self.query
            .push_str(&format!("WHERE {alias}.{field} = '{value}' "));
        self
    }

    pub fn return_fields(mut self, fields: impl AsRef<str>) -> Self {
        self.query.push_str(&format!("RETURN {}", fields.as_ref()));
        self
    }

    pub fn build(self) -> GraphQuery {
        GraphQuery::Cypher(self.query.trim().to_string())
    }
}

impl Default for CypherQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GraphQlQueryBuilder {
    root: String,
    fields: Vec<String>,
    args: Vec<(String, String)>,
}

impl GraphQlQueryBuilder {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            fields: Vec::new(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.push((key.into(), value.into()));
        self
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.fields.push(field.into());
        self
    }

    pub fn build(self) -> GraphQuery {
        let args = if self.args.is_empty() {
            String::new()
        } else {
            let joined = self
                .args
                .into_iter()
                .map(|(k, v)| format!("{k}: \"{v}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({joined})")
        };

        let fields = if self.fields.is_empty() {
            "id".to_string()
        } else {
            self.fields.join(" ")
        };

        GraphQuery::GraphQl(format!(
            "query {{ {}{} {{ {} }} }}",
            self.root, args, fields
        ))
    }
}

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
        Self::new(Arc::new(InMemoryGraphStore::default()))
    }

    pub fn neo4j(
        uri: impl Into<String>,
        user: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(Neo4jGraphStore::new(uri, user, password)))
    }

    pub fn surreal(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self::new(Arc::new(SurrealGraphStore::new(
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
        Self::new(Arc::new(SurrealGraphStore::new_with_auth(
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

#[nova_async_trait]
impl NovaPlugin for NovaGraphDb {
    fn name(&self) -> &'static str {
        "NovaGraphDb"
    }

    async fn on_init(&self) {
        println!("🕸️ Initializing GraphDB Plugin...");
    }

    fn extend_router(&self, router: Router<()>) -> Router<()> {
        router.layer(Extension(self.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, label: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            labels: vec![label.to_string()],
            properties: HashMap::new(),
        }
    }

    fn edge(id: &str, from: &str, to: &str, rel_type: &str) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            rel_type: rel_type.to_string(),
            properties: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn traversal_helpers_work_for_in_memory_graph() {
        let graph = NovaGraphDb::in_memory();

        graph
            .upsert_node(node("u1", "User"))
            .await
            .expect("insert node u1");
        graph
            .upsert_node(node("u2", "User"))
            .await
            .expect("insert node u2");
        graph
            .upsert_node(node("u3", "User"))
            .await
            .expect("insert node u3");
        graph
            .upsert_edge(edge("e1", "u1", "u2", "FOLLOWS"))
            .await
            .expect("insert edge e1");
        graph
            .upsert_edge(edge("e2", "u2", "u3", "FOLLOWS"))
            .await
            .expect("insert edge e2");

        let json = graph
            .traverse_json("u1", 2)
            .await
            .expect("traversal should serialize");

        let nodes = json
            .get("nodes")
            .and_then(JsonValue::as_array)
            .expect("nodes array");
        let edges = json
            .get("edges")
            .and_then(JsonValue::as_array)
            .expect("edges array");

        assert_eq!(nodes.len(), 3);
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    async fn in_memory_rejects_empty_node_id() {
        let store = InMemoryGraphStore::default();
        let result = store
            .upsert_node(GraphNode {
                id: String::new(),
                labels: vec!["User".to_string()],
                properties: HashMap::new(),
            })
            .await;

        assert!(matches!(result, Err(GraphDbError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn in_memory_rejects_invalid_edge_input() {
        let store = InMemoryGraphStore::default();

        let empty_id = store
            .upsert_edge(GraphEdge {
                id: String::new(),
                from: "a".to_string(),
                to: "b".to_string(),
                rel_type: "FOLLOWS".to_string(),
                properties: HashMap::new(),
            })
            .await;
        assert!(matches!(empty_id, Err(GraphDbError::InvalidInput(_))));

        let missing_endpoints = store
            .upsert_edge(GraphEdge {
                id: "e1".to_string(),
                from: "a".to_string(),
                to: "b".to_string(),
                rel_type: "FOLLOWS".to_string(),
                properties: HashMap::new(),
            })
            .await;
        assert!(matches!(
            missing_endpoints,
            Err(GraphDbError::InvalidInput(_))
        ));
    }

    #[tokio::test]
    async fn in_memory_execute_is_not_implemented() {
        let store = InMemoryGraphStore::default();
        let result = store
            .execute(GraphQuery::Cypher("RETURN 1".to_string()))
            .await;
        assert!(matches!(result, Err(GraphDbError::NotImplemented(_))));
    }

    #[test]
    fn cypher_builder_produces_expected_query() {
        let q = CypherQueryBuilder::new()
            .match_node("n", "User")
            .where_eq("n", "id", "u1")
            .return_fields("n")
            .build();

        assert_eq!(
            q,
            GraphQuery::Cypher("MATCH (n:User) WHERE n.id = 'u1' RETURN n".to_string())
        );
    }

    #[test]
    fn graphql_builder_produces_expected_query() {
        let q = GraphQlQueryBuilder::new("users")
            .arg("id", "u1")
            .field("id")
            .field("email")
            .build();

        assert_eq!(
            q,
            GraphQuery::GraphQl("query { users(id: \"u1\") { id email } }".to_string())
        );
    }

    #[tokio::test]
    async fn neo4j_adapter_is_constructible() {
        let graph = NovaGraphDb::neo4j("http://127.0.0.1:65535", "neo4j", "pass");
        let result = graph
            .execute(GraphQuery::Cypher("RETURN 1".to_string()))
            .await;
        assert!(matches!(result, Err(GraphDbError::Backend(_))));
    }

    #[tokio::test]
    async fn neo4j_rejects_graphql_query_type() {
        let graph = NovaGraphDb::neo4j("http://127.0.0.1:65535", "neo4j", "pass");
        let result = graph
            .execute(GraphQuery::GraphQl("query { users { id } }".to_string()))
            .await;
        assert!(matches!(result, Err(GraphDbError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn surreal_adapter_is_constructible() {
        let graph = NovaGraphDb::surreal("http://127.0.0.1:65535", "nova", "main");
        let result = graph
            .execute(GraphQuery::GraphQl("query { ping }".to_string()))
            .await;
        assert!(matches!(result, Err(GraphDbError::Backend(_))));
    }

    #[test]
    fn surreal_helpers_parse_result_rows_and_nodes() {
        let payload = serde_json::json!([
            {
                "status": "OK",
                "result": [
                    {
                        "neighbors": [
                            {
                                "id": {"tb": "node", "id": "u2"},
                                "properties": {"email": "u2@nova.rs"}
                            },
                            {
                                "id": "node:u3",
                                "properties": {"email": "u3@nova.rs"}
                            }
                        ]
                    }
                ]
            }
        ]);

        let rows = surreal_result_rows(&payload);
        assert_eq!(rows.len(), 1);

        let neighbors = rows[0]
            .get("neighbors")
            .and_then(JsonValue::as_array)
            .expect("neighbors should parse");
        assert_eq!(neighbors.len(), 2);

        let n1 = surreal_value_to_node(&neighbors[0]).expect("first node parses");
        let n2 = surreal_value_to_node(&neighbors[1]).expect("second node parses");

        assert_eq!(n1.id, "u2");
        assert_eq!(n2.id, "u3");
    }

    #[test]
    fn surreal_helpers_neighbors_fallback_shape_parses() {
        let payload = serde_json::json!([
            {
                "status": "OK",
                "result": [
                    {
                        "neighbors": [
                            {
                                "id": "node:u7",
                                "properties": {"name": "fallback"}
                            }
                        ]
                    }
                ]
            }
        ]);

        let rows = surreal_result_rows(&payload);
        let neighbors = rows[0]
            .get("neighbors")
            .and_then(JsonValue::as_array)
            .expect("neighbors should exist");

        let parsed = surreal_value_to_node(&neighbors[0]).expect("fallback neighbor shape parses");
        assert_eq!(parsed.id, "u7");
    }

    #[tokio::test]
    async fn graph_to_json_serializes_traversal_output() {
        let graph = NovaGraphDb::in_memory();
        graph
            .upsert_node(GraphNode {
                id: "s1".to_string(),
                labels: vec!["User".to_string()],
                properties: HashMap::new(),
            })
            .await
            .expect("insert node");

        let json = graph.traverse_json("s1", 1).await.expect("traverse json");
        assert!(json.get("nodes").is_some());
        assert!(json.get("edges").is_some());
    }
}
