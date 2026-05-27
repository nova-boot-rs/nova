use nova_nosql::NovaNoSql;

#[tokio::main]
async fn main() {
    // Demonstrate constructing a Redis-backed NovaNoSql wrapper. Requires a
    // running Redis instance at the provided URL for full functionality.
    match NovaNoSql::redis_primary("redis://127.0.0.1:6379", "example").await {
        Ok(nosql) => {
            println!("connected to redis-backed NovaNoSql");
            // Try a dummy upsert & get flow using JSON strings
            let _ = nosql.upsert("demo", "id1", &serde_json::json!({"hello":"world"})).await;
        }
        Err(e) => {
            eprintln!("failed to construct NovaNoSql: {}", e);
        }
    }
}
