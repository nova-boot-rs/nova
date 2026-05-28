use nova_boot_sql::QueryCacheStore;
use std::time::Duration;

#[tokio::test]
async fn redis_cache_set_get_del_ttl() {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());

    match nova_boot_sql::RedisQueryCache::new(&url).await {
        Ok(cache) => {
            // set with short TTL
            cache
                .set("test-key", "value".to_string(), Duration::from_secs(1))
                .await;
            let got: Option<String> = cache.get("test-key").await;
            assert_eq!(got.as_deref(), Some("value"));

            // wait for expiry
            tokio::time::sleep(Duration::from_millis(1100)).await;
            let got2: Option<String> = cache.get("test-key").await;
            assert!(got2.is_none(), "key should expire after TTL");

            // test delete
            cache
                .set("test-key2", "v2".to_string(), Duration::from_secs(60))
                .await;
            cache.del("test-key2").await;
            let got3: Option<String> = cache.get("test-key2").await;
            assert!(got3.is_none(), "key should be deleted");
        }
        Err(e) => {
            eprintln!(
                "Skipping Redis integration test (no Redis available): {}",
                e
            );
            return;
        }
    }
}
