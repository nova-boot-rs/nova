#![cfg(feature = "redis-store")]

use nova_core::{RedisStore, ResilienceStore};
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
#[ignore]
async fn redis_store_basic_flow() {
    let url = env::var("NOVA_TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".into());
    let backend = RedisStore::new(&url).expect("create redis store");
    let store: Arc<dyn ResilienceStore> = Arc::new(backend);

    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let key = format!("test:nova:{}:{}", std::process::id(), now_nanos);

    // ensure clean
    let _ = store.del(&key).await;

    let v1 = store.incr(&key).await.expect("incr 1");
    assert_eq!(v1, 1);

    let v2 = store.incr(&key).await.expect("incr 2");
    assert_eq!(v2, 2);

    let got = store.get_i64(&key).await.expect("get int");
    assert_eq!(got, Some(2));

    // set with TTL
    store.set_ex(&key, 42, 2).await.expect("set_ex");
    let got2 = store.get_i64(&key).await.expect("get after set_ex");
    assert_eq!(got2, Some(42));

    // cleanup
    store.del(&key).await.expect("del");
}
