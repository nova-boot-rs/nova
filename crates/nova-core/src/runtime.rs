use crate::openapi::build_openapi_document;
use crate::response::ApiResponse;
use crate::traits::NovaPlugin;
use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::routing::get;
use axum::routing::MethodRouter;
use axum::{Router, serve};
use nova_observability::{init_tracing, request_id_layer};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

async fn framework_health() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::with_status(
        StatusCode::OK,
        json!({"status": "healthy", "service": "nova"}),
    ))
}

#[derive(Clone)]
struct OpenApiMeta {
    service_name: String,
}

async fn openapi_json(Extension(meta): Extension<OpenApiMeta>) -> Json<serde_json::Value> {
    Json(build_openapi_document(&meta.service_name))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

pub struct NovaApp<S = ()>
where
    S: Clone + Send + Sync + 'static,
{
    name: &'static str,
    port: u16,
    router: Router<S>,
    address: std::net::SocketAddr,
    state: S,
    plugins: Vec<Box<dyn NovaPlugin>>,
}

pub struct NovaRoute {
    pub path: &'static str,
    pub method: &'static str,
    pub handler: fn() -> MethodRouter<()>,
}

inventory::collect!(NovaRoute);

impl<S> NovaApp<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(name: &'static str, port: u16, state: S) -> Self {
        let router = Router::<S>::new()
            .route("/health", get(framework_health))
            .route("/openapi.json", get(openapi_json))
            .layer(request_id_layer())
            .layer(TraceLayer::new_for_http());

        Self {
            name,
            port,
            router,
            address: format!("0.0.0.0:{port}").parse().expect("Invalid address"),
            state,
            plugins: Vec::new(),
        }
    }

    pub fn add_plugin<P: NovaPlugin + 'static>(mut self, plugin: P) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    pub async fn run(self) {
        init_tracing(self.name);

        for plugin in &self.plugins {
            info!("🔌 Loading plugin: {}", plugin.name());
            plugin.on_init().await;
        }

        let mut app_router = self.router.with_state(self.state.clone());

        for route in inventory::iter::<NovaRoute> {
            info!("📡 Registering {} route: {}", route.method, route.path);
            let method_router = (route.handler)();
            app_router = app_router.route(route.path, method_router);
        }

        let mut final_router = app_router
            .layer(axum::Extension(self.state.clone()))
            .layer(axum::Extension(OpenApiMeta {
                service_name: self.name.to_string(),
            }));

        for plugin in &self.plugins {
            info!("🔌 Injecting state for: {}", plugin.name());
            final_router = plugin.extend_router(final_router);
        }

        info!("🚀 {{{}}} starting on port {}", self.name, self.port);
        let listener = TcpListener::bind(&self.address)
            .await
            .expect("Failed to bind server socket");

        serve(listener, final_router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .expect("Server failed to start");

        info!("🛑 {{{}}} shutting down", self.name);
        for plugin in self.plugins.iter().rev() {
            info!("🔌 Stopping plugin: {}", plugin.name());
            plugin.on_shutdown().await;
        }
    }
}
