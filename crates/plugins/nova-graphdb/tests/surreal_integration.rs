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
    let username = std::env::var("SURREALDB_USER").unwrap_or_else(|_| "root".to_string());
    let password = std::env::var("SURREALDB_PASSWORD").unwrap_or_else(|_| "root".to_string());

    let root_client = reqwest::Client::new();
    let mut root_headers = reqwest::header::HeaderMap::new();
    root_headers.insert(
        reqwest::header::ACCEPT,
        "application/json".parse().expect("accept header"),
    );

    let create_namespace = root_client
        .post(format!("{}/sql", endpoint.trim_end_matches('/')))
        .basic_auth(&username, Some(&password))
        .headers(root_headers)
        .body(format!("DEFINE NAMESPACE `{namespace}`"))
        .send()
        .await
        .expect("create namespace request should succeed");
    assert!(
        create_namespace.status().is_success(),
        "create namespace request failed: {}",
        create_namespace.text().await.expect("namespace body")
    );

    let mut ns_headers = reqwest::header::HeaderMap::new();
    ns_headers.insert("Accept", "application/json".parse().expect("accept header"));
    ns_headers.insert("surreal-ns", namespace.parse().expect("namespace header"));

    let create_database = root_client
        .post(format!("{}/sql", endpoint.trim_end_matches('/')))
        .basic_auth(&username, Some(&password))
        .headers(ns_headers)
        .body(format!("DEFINE DATABASE `{database}`"))
        .send()
        .await
        .expect("create database request should succeed");
    assert!(
        create_database.status().is_success(),
        "create database request failed: {}",
        create_database.text().await.expect("database body")
    );

    let graph = NovaGraphDb::surreal_with_auth(endpoint, namespace, database, username, password);

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
