use nova_core::Json;
use nova_core::axum::body::Body;
use nova_core::axum::http::Request;
use nova_core::axum::http::StatusCode;
use nova_core::axum::middleware::Next;
use nova_core::axum::response::{IntoResponse, Response};
use nova_core::resilience::{CircuitBreakerBackend, RateLimiterBackend};
use nova_core::{Bulkhead, CircuitBreaker, RateLimiter};
use serde_json::json;
use std::sync::Arc;

/// Rejects requests when the in-memory circuit is open.
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

    let status = resp.status();
    if status.is_server_error() {
        state.record_failure().await;
    } else {
        state.record_success().await;
    }

    resp
}

/// Rejects requests when a circuit backend reports open.
pub async fn circuit_breaker_middleware_boxed(
    state: Arc<dyn CircuitBreakerBackend>,
    req: Request<Body>,
    next: Next,
) -> Response {
    match state.allow().await {
        Ok(allowed) => {
            if !allowed {
                let body = Json(
                    json!({"error": "circuit_open", "message": "service temporarily unavailable"}),
                );
                return (StatusCode::SERVICE_UNAVAILABLE, body).into_response();
            }
        }
        Err(_) => {
            let body =
                Json(json!({"error": "internal_error", "message": "resilience backend error"}));
            return (StatusCode::INTERNAL_SERVER_ERROR, body).into_response();
        }
    }

    let resp = next.run(req).await;

    let status = resp.status();
    if status.is_server_error() {
        let _ = state.record_failure().await;
    } else {
        let _ = state.record_success().await;
    }

    resp
}

/// Token-bucket style in-memory rate limiter by `x-client-id`.
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

/// Rate limiter backed by a `RateLimiterBackend` implementation.
pub async fn rate_limiter_middleware_boxed(
    state: Arc<dyn RateLimiterBackend>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let key = req
        .headers()
        .get("x-client-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();

    match state.allow(&key).await {
        Ok(true) => next.run(req).await,
        Ok(false) => {
            let body =
                Json(json!({"error": "too_many_requests", "message": "rate limit exceeded"}));
            (StatusCode::TOO_MANY_REQUESTS, body).into_response()
        }
        Err(_) => {
            let body =
                Json(json!({"error": "internal_error", "message": "resilience backend error"}));
            (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
        }
    }
}

/// Semaphore-based concurrency limiter.
pub async fn bulkhead_middleware(state: Arc<Bulkhead>, req: Request<Body>, next: Next) -> Response {
    state
        .with_permit(|| async move { next.run(req).await })
        .await
}
