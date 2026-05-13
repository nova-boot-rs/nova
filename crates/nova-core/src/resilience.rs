use axum::Json;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

/// Simple in-memory circuit breaker with manual record API.
#[derive(Debug)]
pub struct CircuitBreaker {
    failures: Arc<Mutex<u32>>,
    threshold: u32,
    open_until: Arc<Mutex<Option<Instant>>>,
    open_duration: Duration,
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
    inner: Arc<Mutex<HashMap<String, (f64, Instant, f64)>>>,
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

/// Middleware: rejects requests when circuit is open, otherwise forwards and records success/failure.
pub async fn circuit_breaker_middleware(
    state: Arc<CircuitBreaker>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if !state.allow().await {
        let body =
            Json(json!({"error": "circuit_open", "message": "service temporarily unavailable"}));
        return (StatusCode::SERVICE_UNAVAILABLE, body).into_response();
    }

    let resp = next.run(req).await;

    // record based on status
    let status = resp.status();
    if status.is_server_error() {
        state.record_failure().await;
    } else {
        state.record_success().await;
    }

    resp
}

/// Middleware: simple token-bucket rate limiting by `x-client-id` header.
pub async fn rate_limiter_middleware(
    state: Arc<RateLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let key = req
        .headers()
        .get("x-client-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();

    if !state.allow(&key, 1.0).await {
        let body = Json(json!({"error": "too_many_requests", "message": "rate limit exceeded"}));
        return (StatusCode::TOO_MANY_REQUESTS, body).into_response();
    }

    next.run(req).await
}

/// Middleware: bulkhead/semaphore-based concurrency limiter.
pub async fn bulkhead_middleware(state: Arc<Bulkhead>, req: Request<Body>, next: Next) -> Response {
    state
        .with_permit(|| async move { next.run(req).await })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

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
