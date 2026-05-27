use nova_graphdb::{NovaGraphDb, GraphNode, GraphEdge};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // Construct an in-memory graph store and upsert a couple of nodes/edges.
    let graph = NovaGraphDb::in_memory();

    let mut props = HashMap::new();
    props.insert("name".to_string(), serde_json::json!("Alice"));
    let node = GraphNode {
        id: "n1".to_string(),
        labels: vec!["Person".to_string()],
        properties: props,
    };

    graph.upsert_node(node).await.unwrap();

    let mut props2 = HashMap::new();
    props2.insert("name".to_string(), serde_json::json!("Bob"));
    let node2 = GraphNode {
        id: "n2".to_string(),
        labels: vec!["Person".to_string()],
        properties: props2,
    };

    graph.upsert_node(node2).await.unwrap();

    let edge = GraphEdge {
        id: "e1".to_string(),
        from: "n1".to_string(),
        to: "n2".to_string(),
        rel_type: "FRIEND".to_string(),
        properties: HashMap::new(),
    };

    graph.upsert_edge(edge).await.unwrap();

    let sub = graph.traverse_json("n1", 2).await.unwrap();
    println!("subgraph: {}", sub);
}
