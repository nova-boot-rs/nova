use crate::traits::NovaPlugin;
use axum::Json;
use axum::extract::Extension;
use axum::routing::MethodRouter;
use axum::routing::get;
use axum::{Router, serve};
use nova_observability::{build_openapi_document, init_tracing};
use serde_json::json;
use std::collections::HashMap;
use tokio::net::TcpListener;
use tracing::info;

async fn framework_health() -> Json<serde_json::Value> {
    Json(json!({"status": "healthy", "service": "nova"}))
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
    pub handler: fn() -> MethodRouter<()>, // Keep () — converts to S via Into
}

inventory::collect!(NovaRoute);

type RouteRegistry = HashMap<(&'static str, &'static str), fn() -> MethodRouter<()>>;

impl<S> NovaApp<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(name: &'static str, port: u16, state: S) -> Self {
        let router = Router::<S>::new()
            .route("/health", get(framework_health))
            .route("/openapi.json", get(openapi_json));

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

        // Start with framework routes + state
        let mut app_router = self.router.with_state(self.state.clone());

        // Collect and deduplicate inventory routes
        let mut route_map: RouteRegistry = HashMap::new();

        for route in inventory::iter::<NovaRoute> {
            let key = (route.method, route.path);
            if route_map.insert(key, route.handler).is_some() {
                tracing::warn!(
                    "Duplicate route detected, overriding: {} {}",
                    route.method,
                    route.path
                );
            }
        }

        for ((method, path), handler) in route_map.into_iter() {
            info!("📡 Registering {} route: {}", method, path);
            let method_router: MethodRouter<()> = (handler)();
            // MethodRouter<()> → MethodRouter<S> via Into
            app_router = app_router.route(path, method_router);
        }

        // Add OpenAPI metadata as Extension (separate from app state)
        let mut final_router = app_router.layer(axum::Extension(OpenApiMeta {
            service_name: self.name.to_string(),
        }));

        // Let plugins extend the router
        for plugin in &self.plugins {
            info!("🔌 Injecting state for: {}", plugin.name());
            final_router = plugin.extend_router(final_router);
        }

        info!("🚀 {} starting on port {}", self.name, self.port);
        let listener = TcpListener::bind(&self.address)
            .await
            .expect("Failed to bind server socket");

        serve(listener, final_router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .expect("Server failed to start");

        info!("🛑 {} shutting down", self.name);
        for plugin in self.plugins.iter().rev() {
            info!("🔌 Stopping plugin: {}", plugin.name());
            plugin.on_shutdown().await;
        }
    }
}
