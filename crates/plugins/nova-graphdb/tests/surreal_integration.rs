use nova_graphdb::{GraphEdge, GraphNode, NovaGraphDb};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn surreal_end_to_end_traversal() {
    let endpoint = match std::env::var("SURREALDB_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Skipping Surreal integration test: SURREALDB_URL is not set");
            return;
        }
    };

    let namespace = std::env::var("SURREALDB_NS").unwrap_or_else(|_| "nova".to_string());
    let database = std::env::var("SURREALDB_DB").unwrap_or_else(|_| "main".to_string());

    let graph = NovaGraphDb::surreal(endpoint, namespace, database);

    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let n1 = format!("int_u1_{suffix}");
    let n2 = format!("int_u2_{suffix}");

    let node1 = GraphNode {
        id: n1.clone(),
        labels: vec!["node".to_string()],
        properties: HashMap::from([(
            "email".to_string(),
            JsonValue::String("u1@nova.rs".to_string()),
        )]),
    };

    let node2 = GraphNode {
        id: n2.clone(),
        labels: vec!["node".to_string()],
        properties: HashMap::from([(
            "email".to_string(),
            JsonValue::String("u2@nova.rs".to_string()),
        )]),
    };

    graph
        .upsert_node(node1)
        .await
        .expect("upsert node1 should succeed");
    graph
        .upsert_node(node2)
        .await
        .expect("upsert node2 should succeed");

    let edge = GraphEdge {
        id: format!("int_e_{suffix}"),
        from: n1.clone(),
        to: n2.clone(),
        rel_type: "FOLLOWS".to_string(),
        properties: HashMap::from([("since".to_string(), JsonValue::Number(2024.into()))]),
    };

    graph
        .upsert_edge(edge)
        .await
        .expect("upsert edge should succeed");

    let traversal = graph
        .traverse_json(&n1, 1)
        .await
        .expect("traverse_json should succeed");

    let nodes = traversal
        .get("nodes")
        .and_then(JsonValue::as_array)
        .expect("nodes array in traversal response");
    let edges = traversal
        .get("edges")
        .and_then(JsonValue::as_array)
        .expect("edges array in traversal response");

    assert!(
        nodes
            .iter()
            .any(|n| n.get("id") == Some(&JsonValue::String(n1.clone()))),
        "expected traversal to contain start node"
    );
    assert!(
        nodes
            .iter()
            .any(|n| n.get("id") == Some(&JsonValue::String(n2.clone()))),
        "expected traversal to contain neighbor node"
    );
    assert!(
        edges
            .iter()
            .any(|e| e.get("from") == Some(&JsonValue::String(n1.clone()))
                && e.get("to") == Some(&JsonValue::String(n2.clone()))),
        "expected traversal to contain connecting edge"
    );
}
