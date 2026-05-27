use nova_sql::{NovaSql, cache::InMemoryQueryCache};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let cache: Arc<dyn nova_sql::cache::QueryCacheStore> = Arc::new(InMemoryQueryCache::default());

    let sql = NovaSql::connect("sqlite::memory:", false)
        .await
        .with_cache_store(cache);

    // Use cached_json with a fetcher that returns a simple JSON value.
    let value = sql
        .cached_json("demo:key", Duration::from_secs(30), |_db| async move {
            Ok(serde_json::json!({"from_db": true}))
        })
        .await
        .expect("fetch should succeed");

    println!("cached value: {}", value);
}
