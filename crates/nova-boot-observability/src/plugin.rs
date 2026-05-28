//! Observability plugin wiring for Nova applications.
//!
//! Adds an `/openapi.json` endpoint, initializes tracing on startup, and
//! installs request-context middleware and HTTP tracing layers.
use axum::Json;
use axum::middleware;
use axum::routing::get;
use nova_boot::{NovaPlugin, async_trait, axum::Router};
use tower_http::trace::TraceLayer;

use crate::{attach_request_context, build_openapi_document, init_tracing, request_id_layer};

/// Small plugin that wires the observability stack into the application.
pub struct ObservabilityPlugin {
    pub service_name: &'static str,
}

impl ObservabilityPlugin {
    /// Create a new `ObservabilityPlugin` for `service_name`.
    pub fn new(service_name: &'static str) -> Self {
        Self { service_name }
    }
}

#[async_trait]
impl NovaPlugin for ObservabilityPlugin {
    fn name(&self) -> &'static str {
        "observability"
    }

    async fn on_init(&self) {
        init_tracing(self.service_name);
    }

    async fn on_shutdown(&self) {}

    fn extend_router(&self, router: Router) -> Router {
        let svc = self.service_name;

        router
            .route(
                "/openapi.json",
                get(move || async move { Json(build_openapi_document(svc)) }),
            )
            .route_layer(middleware::from_fn(attach_request_context))
            .layer(request_id_layer())
            .layer(TraceLayer::new_for_http())
    }
}
