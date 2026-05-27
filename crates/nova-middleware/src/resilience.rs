//! Resilience primitives: circuit breakers, rate limiters and retry policies.
//!
//! This module provides both in-memory and distributed implementations
//! of common resilience patterns used by services to remain available
//! under failure conditions.

use async_trait::async_trait;
use nova_boot::{
    NovaError,
    config::{CircuitBreakerConfig, RateLimiterConfig, ResilienceBackend},
};
use nova_resilience_store::ResilienceStore;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

type TokenBucketState = (f64, Instant, f64);

/// Simple in-memory circuit breaker with manual record API.
///
/// Use `CircuitBreaker::allow()` to check whether calls are permitted and
/// `record_failure()` / `record_success()` to update internal counters.
#[derive(Debug)]
pub struct CircuitBreaker {
    failures: Arc<Mutex<u32>>,
    threshold: u32,
    open_until: Arc<Mutex<Option<Instant>>>,
    open_duration: Duration,
}

/// Circuit breaker that stores state in a distributed store (Redis, etc.)
///
/// This implementation delegates state storage to a `ResilienceStore`
/// (Lua/Redis) and is suitable for multi-instance deployments.
#[derive(Clone)]
pub struct DistributedCircuitBreaker {
    store: Arc<dyn ResilienceStore>,
    name: String,
    threshold: u32,
    open_ttl_seconds: usize,
}

impl DistributedCircuitBreaker {
    pub fn new(
        store: Arc<dyn ResilienceStore>,
        name: impl Into<String>,
        threshold: u32,
        open_ttl_seconds: usize,
    ) -> Self {
        Self {
            store,
            name: name.into(),
            threshold,
            open_ttl_seconds,
        }
    }

    fn failures_key(&self) -> String {
        format!("cb:fail:{}", self.name)
    }
    fn open_key(&self) -> String {
        format!("cb:open:{}", self.name)
    }

    pub async fn allow(&self) -> Result<bool, NovaError> {
        if let Some(v) = self.store.get_i64(&self.open_key()).await?
            && v > 0
        {
            return Ok(false);
        }
        Ok(true)
    }

    pub async fn record_failure(&self) -> Result<(), NovaError> {
        let f = self.store.incr(&self.failures_key()).await? as u32;
        if f >= self.threshold {
            // open the breaker for TTL seconds
            self.store
                .set_ex(&self.open_key(), 1, self.open_ttl_seconds)
                .await?;
            // reset failures
            self.store.del(&self.failures_key()).await?;
        }
        Ok(())
    }

    pub async fn record_success(&self) -> Result<(), NovaError> {
        // clear counters and open flag
        self.store.del(&self.failures_key()).await?;
        self.store.del(&self.open_key()).await?;
        Ok(())
    }
}

/// Distributed rate limiter using fixed window counters in the store.
#[derive(Clone)]
pub struct DistributedRateLimiter {
    store: Arc<dyn ResilienceStore>,
    prefix: String,
    capacity: i64,
    window_seconds: usize,
}

impl DistributedRateLimiter {
    pub fn new(
        store: Arc<dyn ResilienceStore>,
        prefix: impl Into<String>,
        capacity: i64,
        window_seconds: usize,
    ) -> Self {
        Self {
            store,
            prefix: prefix.into(),
            capacity,
            window_seconds,
        }
    }

    fn key_for(&self, client: &str) -> String {
        format!("rl:{}:{}", self.prefix, client)
    }

    fn allow_script() -> &'static str {
        r#"
        local key = KEYS[1]
        local window_seconds = tonumber(ARGV[1])
        local capacity = tonumber(ARGV[2])

        local current = redis.call('INCR', key)
        if current == 1 then
            redis.call('EXPIRE', key, window_seconds)
        end

        if current > capacity then
            return 0
        end

        return 1
        "#
    }

    /// Attempt to consume a single token for `client`.
    pub async fn allow(&self, client: &str) -> Result<bool, NovaError> {
        let key = self.key_for(client);
        let window_seconds = self.window_seconds.to_string();
        let capacity = self.capacity.to_string();

        let value = self
            .store
            .eval_lua(
                Self::allow_script(),
                &[key.as_str()],
                &[window_seconds.as_str(), capacity.as_str()],
            )
            .await?;

        match value.as_i64() {
            Some(result) => Ok(result > 0),
            None => Err(NovaError::InternalError(
                "rate limiter lua script returned unexpected value".to_string(),
            )),
        }
    }
}

/// Trait abstraction for circuit breaker backends (in-memory or distributed).
#[async_trait]
pub trait CircuitBreakerBackend: Send + Sync + 'static {
    async fn allow(&self) -> Result<bool, NovaError>;
    async fn record_failure(&self) -> Result<(), NovaError>;
    async fn record_success(&self) -> Result<(), NovaError>;
}

#[async_trait]
impl CircuitBreakerBackend for CircuitBreaker {
    async fn allow(&self) -> Result<bool, NovaError> {
        Ok(self.allow().await)
    }
    async fn record_failure(&self) -> Result<(), NovaError> {
        self.record_failure().await;
        Ok(())
    }
    async fn record_success(&self) -> Result<(), NovaError> {
        self.record_success().await;
        Ok(())
    }
}

#[async_trait]
impl CircuitBreakerBackend for DistributedCircuitBreaker {
    async fn allow(&self) -> Result<bool, NovaError> {
        self.allow().await
    }
    async fn record_failure(&self) -> Result<(), NovaError> {
        self.record_failure().await
    }
    async fn record_success(&self) -> Result<(), NovaError> {
        self.record_success().await
    }
}

/// Trait abstraction for rate limiter backends.
#[async_trait]
pub trait RateLimiterBackend: Send + Sync + 'static {
    async fn allow(&self, client: &str) -> Result<bool, NovaError>;
}

#[async_trait]
impl RateLimiterBackend for RateLimiter {
    async fn allow(&self, client: &str) -> Result<bool, NovaError> {
        Ok(self.allow(client, 1.0).await)
    }
}

#[async_trait]
impl RateLimiterBackend for DistributedRateLimiter {
    async fn allow(&self, client: &str) -> Result<bool, NovaError> {
        self.allow(client).await
    }
}

/// Build a `CircuitBreakerBackend` from `CircuitBreakerConfig` and an optional distributed store.
///
/// Helper used by application startup code to create either a local or
/// distributed backend depending on configuration.
pub fn build_circuit_breaker_backend(
    name: &str,
    cfg: &CircuitBreakerConfig,
    store_opt: Option<Arc<dyn ResilienceStore>>,
) -> Arc<dyn CircuitBreakerBackend> {
    match &cfg.backend {
        ResilienceBackend::Local => Arc::new(CircuitBreaker::new(
            cfg.threshold,
            Duration::from_secs(cfg.open_ttl_seconds as u64),
        )),
        ResilienceBackend::Redis { .. } => {
            if let Some(store) = store_opt {
                Arc::new(DistributedCircuitBreaker::new(
                    store,
                    name.to_string(),
                    cfg.threshold,
                    cfg.open_ttl_seconds,
                ))
            } else {
                // fallback to local if no store provided
                Arc::new(CircuitBreaker::new(
                    cfg.threshold,
                    Duration::from_secs(cfg.open_ttl_seconds as u64),
                ))
            }
        }
    }
}

/// Build a `RateLimiterBackend` from `RateLimiterConfig` and an optional distributed store.
///
/// Creates either a local in-memory rate limiter or a distributed backed
/// by `ResilienceStore` depending on configuration.
pub fn build_rate_limiter_backend(
    prefix: &str,
    cfg: &RateLimiterConfig,
    store_opt: Option<Arc<dyn ResilienceStore>>,
) -> Arc<dyn RateLimiterBackend> {
    match &cfg.backend {
        ResilienceBackend::Local => Arc::new(RateLimiter::new(
            cfg.capacity as f64,
            cfg.capacity as f64 / cfg.window_seconds as f64,
        )),
        ResilienceBackend::Redis {
            prefix: cfg_prefix, ..
        } => {
            if let Some(store) = store_opt {
                let pfx = cfg_prefix.clone().unwrap_or_else(|| prefix.to_string());
                Arc::new(DistributedRateLimiter::new(
                    store,
                    pfx,
                    cfg.capacity,
                    cfg.window_seconds,
                ))
            } else {
                Arc::new(RateLimiter::new(
                    cfg.capacity as f64,
                    cfg.capacity as f64 / cfg.window_seconds as f64,
                ))
            }
        }
    }
}

impl CircuitBreaker {
    pub fn new(threshold: u32, open_duration: Duration) -> Self {
        Self {
            failures: Arc::new(Mutex::new(0)),
            threshold,
            open_until: Arc::new(Mutex::new(None)),
            open_duration,
        }
    }

    /// Check whether calls are currently allowed.
    pub async fn allow(&self) -> bool {
        if let Some(until) = *self.open_until.lock().await {
            if Instant::now() < until {
                return false;
            } else {
                // reset
                *self.failures.lock().await = 0;
                *self.open_until.lock().await = None;
                return true;
            }
        }

        true
    }

    /// Record a successful call (resets failure counter).
    pub async fn record_success(&self) {
        *self.failures.lock().await = 0;
    }

    /// Record a failed call and open the breaker if threshold reached.
    pub async fn record_failure(&self) {
        let mut f = self.failures.lock().await;
        *f += 1;
        if *f >= self.threshold {
            *self.open_until.lock().await = Some(Instant::now() + self.open_duration);
        }
    }
}

/// Simple retry policy with exponential backoff and jitter.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: usize,
    pub base_delay: Duration,
}

impl RetryPolicy {
    pub fn new(max_retries: usize, base_delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay,
        }
    }

    pub fn next_delay(&self, attempt: usize) -> Duration {
        let exp = 2u64.pow(attempt as u32);
        let millis = self.base_delay.as_millis() as u64 * exp;
        Duration::from_millis(millis)
    }

    /// Retry an async closure using the policy. The closure returns Result<T, E>.
    pub async fn retry_async<F, Fut, T, E>(&self, mut op: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let mut attempt = 0usize;
        loop {
            let res = op().await;
            match res {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if attempt >= self.max_retries {
                        return Err(e);
                    }

                    let delay = self.next_delay(attempt);
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }
}

/// Bulkhead pattern using a semaphore to limit concurrent operations.
#[derive(Clone)]
pub struct Bulkhead {
    semaphore: Arc<Semaphore>,
}

impl Bulkhead {
    pub fn new(limit: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(limit)),
        }
    }

    /// Acquire a permit asynchronously and run the closure while the permit is held.
    pub async fn with_permit<F, Fut, T>(&self, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let permit = self.semaphore.acquire().await.unwrap();
        let out = f().await;
        drop(permit);
        out
    }
}

/// Very small in-memory token bucket rate limiter keyed by string (e.g. client id).
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, TokenBucketState>>>,
    capacity: f64,
    refill_rate_per_sec: f64,
}

impl RateLimiter {
    pub fn new(capacity: f64, refill_rate_per_sec: f64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            capacity,
            refill_rate_per_sec,
        }
    }

    /// Attempt to take `amount` tokens for `key`. Returns true if allowed.
    pub async fn allow(&self, key: &str, amount: f64) -> bool {
        let mut map = self.inner.lock().await;
        let now = Instant::now();
        let entry =
            map.entry(key.to_string())
                .or_insert((self.capacity, now, self.refill_rate_per_sec));

        // refill
        let elapsed = now.duration_since(entry.1).as_secs_f64();
        entry.0 = (entry.0 + elapsed * entry.2).min(self.capacity);
        entry.1 = now;

        if entry.0 >= amount {
            entry.0 -= amount;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_resilience_store::{LuaValue, ResilienceError, ResilienceStore};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct MockLuaStore {
        response: LuaValue,
        script: Arc<Mutex<Option<String>>>,
        keys: Arc<Mutex<Vec<String>>>,
        args: Arc<Mutex<Vec<String>>>,
    }

    impl MockLuaStore {
        fn new(response: LuaValue) -> Self {
            Self {
                response,
                script: Arc::new(Mutex::new(None)),
                keys: Arc::new(Mutex::new(Vec::new())),
                args: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl ResilienceStore for MockLuaStore {
        async fn incr(&self, _key: &str) -> Result<i64, ResilienceError> {
            unimplemented!()
        }

        async fn get_i64(&self, _key: &str) -> Result<Option<i64>, ResilienceError> {
            unimplemented!()
        }

        async fn set_ex(
            &self,
            _key: &str,
            _val: i64,
            _ttl_seconds: usize,
        ) -> Result<(), ResilienceError> {
            unimplemented!()
        }

        async fn del(&self, _key: &str) -> Result<(), ResilienceError> {
            unimplemented!()
        }

        async fn eval_lua(
            &self,
            script: &str,
            keys: &[&str],
            args: &[&str],
        ) -> Result<LuaValue, ResilienceError> {
            *self.script.lock().await = Some(script.to_string());
            *self.keys.lock().await = keys.iter().map(|value| (*value).to_string()).collect();
            *self.args.lock().await = args.iter().map(|value| (*value).to_string()).collect();
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn distributed_rate_limiter_uses_atomic_lua_script() {
        let store_impl = Arc::new(MockLuaStore::new(LuaValue::Integer(1)));
        let store: Arc<dyn ResilienceStore> = store_impl.clone();
        let limiter = DistributedRateLimiter::new(store, "api", 10, 60);

        assert!(limiter.allow("client1").await.unwrap());

        let script = store_impl.script.lock().await.clone().unwrap();
        assert!(script.contains("redis.call('INCR', key)"));
        assert!(script.contains("redis.call('EXPIRE', key, window_seconds)"));
        assert_eq!(store_impl.keys.lock().await.as_slice(), &["rl:api:client1"]);
        assert_eq!(store_impl.args.lock().await.as_slice(), &["60", "10"]);
    }

    #[tokio::test]
    async fn circuit_breaker_opens_at_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1));

        assert!(cb.allow().await);

        cb.record_failure().await;
        assert!(cb.allow().await);

        cb.record_failure().await;
        assert!(cb.allow().await);

        cb.record_failure().await;
        assert!(!cb.allow().await);
    }

    #[tokio::test]
    async fn retry_policy_retries() {
        let policy = RetryPolicy::new(2, Duration::from_millis(1));
        let cnt = std::sync::Arc::new(tokio::sync::Mutex::new(0usize));
        let cnt_clone = cnt.clone();

        let res: Result<usize, ()> = policy
            .retry_async(|| {
                let cnt_inner = cnt_clone.clone();
                async move {
                    let mut lock = cnt_inner.lock().await;
                    *lock += 1;
                    if *lock >= 2 { Ok(42) } else { Err(()) }
                }
            })
            .await;

        assert_eq!(res.unwrap(), 42);
    }

    #[tokio::test]
    async fn bulkhead_limits() {
        let bh = Bulkhead::new(1);
        let b = bh.clone();

        let t1 = tokio::spawn(async move {
            b.with_permit(|| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                1
            })
            .await
        });

        // second concurrent attempt should block until permit released, but still complete
        let b2 = bh.clone();
        let t2 = tokio::spawn(async move { b2.with_permit(|| async { 2 }).await });

        let (r1, r2) = tokio::join!(t1, t2);
        assert_eq!(r1.unwrap(), 1);
        assert_eq!(r2.unwrap(), 2);
    }

    #[tokio::test]
    async fn rate_limiter_allows_and_blocks() {
        let rl = RateLimiter::new(2.0, 1.0);
        let key = "client1";

        assert!(rl.allow(key, 1.0).await);
        assert!(rl.allow(key, 1.0).await);
        assert!(!rl.allow(key, 1.0).await);
    }
}
