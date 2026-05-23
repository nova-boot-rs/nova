use crate::{cache::{InMemoryQueryCache, QueryCacheStore}};
use std::time::Duration;

#[tokio::test]
async fn in_memory_query_cache_expires_entries() {
    let cache = InMemoryQueryCache::default();

    cache
        .set(
            "users:1",
            "{\"id\":1}".to_string(),
            Duration::from_millis(10),
        )
        .await;

    let hit = cache.get("users:1").await;
    assert!(hit.is_some());

    tokio::time::sleep(Duration::from_millis(20)).await;
    let miss = cache.get("users:1").await;
    assert!(miss.is_none());
}