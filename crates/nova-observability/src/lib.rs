use axum::extract::Request;
use axum::http::header::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Once;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

pub mod openapi;

pub use openapi::{OpenApiHook, build_openapi_document};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);
static TRACING_INIT: Once = Once::new();

pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: RequestId,
    pub started_at: Instant,
}

#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub service_name: &'static str,
    pub emit_request_logs: bool,
    pub request_id_header: &'static str,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "nova",
            emit_request_logs: true,
            request_id_header: REQUEST_ID_HEADER,
        }
    }
}

pub trait NovaMetricsRecorder: Send + Sync {
    fn record_request_duration(&self, _route: &str, _status: u16, _duration: Duration) {}

    fn record_error(&self, _route: &str, _kind: &str) {}

    fn record_db_query_duration(&self, _name: &str, _duration: Duration) {}
}

#[derive(Debug, Default, Clone)]
pub struct NoopMetricsRecorder;

impl NovaMetricsRecorder for NoopMetricsRecorder {}

pub fn init_tracing(service_name: &str) {
    let service_name = service_name.to_owned();

    TRACING_INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_ansi(true)
            .with_writer(std::io::stdout)
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .init();

        info!(service = %service_name, "observability initialized");
    });
}

pub fn next_request_id() -> RequestId {
    let raw = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    RequestId(format!("req-{raw:016x}"))
}

pub fn request_id_header_name() -> HeaderName {
    HeaderName::from_static(REQUEST_ID_HEADER)
}

pub fn request_id_layer() -> SetRequestIdLayer<MakeRequestUuid> {
    SetRequestIdLayer::new(request_id_header_name(), MakeRequestUuid)
}

pub fn request_context_from_request() -> RequestContext {
    RequestContext {
        request_id: next_request_id(),
        started_at: Instant::now(),
    }
}

pub async fn attach_request_context(mut request: Request, next: Next) -> Response {
    let context = request_context_from_request();
    let header_value = HeaderValue::from_str(&context.request_id.0)
        .unwrap_or_else(|_| HeaderValue::from_static("invalid-request-id"));

    request.extensions_mut().insert(context.clone());

    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(request_id_header_name(), header_value);

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_monotonic_request_ids() {
        let first = next_request_id();
        let second = next_request_id();

        assert!(first.0.starts_with("req-"));
        assert!(second.0.starts_with("req-"));
        assert_ne!(first.0, second.0);
    }
}
