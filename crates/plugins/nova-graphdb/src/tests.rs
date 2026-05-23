use crate::{
    builders::{CypherQueryBuilder, GraphQlQueryBuilder},
    error::GraphDbError,
    memory::InMemoryGraphStore,
    traits::GraphStore,
    surreal::{surreal_result_rows, surreal_value_to_node},
    types::*,
    NovaGraphDb,
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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

    graph.upsert_node(node("u1", "User")).await.expect("insert node u1");
    graph.upsert_node(node("u2", "User")).await.expect("insert node u2");
    graph.upsert_node(node("u3", "User")).await.expect("insert node u3");
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
    assert!(matches!(missing_endpoints, Err(GraphDbError::InvalidInput(_))));
}

#[tokio::test]
async fn in_memory_execute_is_not_implemented() {
    let store = InMemoryGraphStore::default();
    let result = store.execute(GraphQuery::Cypher("RETURN 1".to_string())).await;
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
    let result = graph.execute(GraphQuery::Cypher("RETURN 1".to_string())).await;
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