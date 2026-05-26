use crate::traits::NovaPlugin;
use axum::Json;
use axum::routing::MethodRouter;
use axum::routing::get;
use axum::{Router, serve};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

async fn framework_health() -> Json<serde_json::Value> {
    Json(json!({"status": "healthy", "service": "nova"}))
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
    router: Router<()>,
    address: SocketAddr,
    state: S,
    plugins: Vec<Box<dyn NovaPlugin>>,
}

pub struct NovaRoute {
    pub path: &'static str,
    pub method: &'static str,
    pub handler: fn() -> MethodRouter<()>,
}

inventory::collect!(NovaRoute);

type RouteRegistry = HashMap<(&'static str, &'static str), fn() -> MethodRouter<()>>;

impl<S> NovaApp<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new(name: &'static str, port: u16, state: S) -> Self {
        let router = Router::<()>::new().route("/health", get(framework_health));

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

    async fn build_router(&self) -> Router<()> {
        // Step 1: Initialize all plugins
        for plugin in &self.plugins {
            info!("🔌 Loading plugin: {}", plugin.name());
            plugin.on_init().await;
        }

        // Step 2: Build base router as Router<()> with framework routes
        let mut base: Router<()> = self.router.clone();

        // Step 3: Collect and deduplicate inventory routes
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

        // Step 4: Register inventory routes
        for ((method, path), handler) in route_map.into_iter() {
            info!("📡 Registering {} route: {}", method, path);
            let method_router: MethodRouter<()> = (handler)();
            base = base.route(path, method_router);
        }

        // Step 5: Inject application state as an Extension layer
        // Handlers use `Extension<S>` or a wrapper to access state.
        base = base.layer(axum::Extension(self.state.clone()));

        // Step 6: Let plugins extend the router
        for plugin in &self.plugins {
            info!("🔌 Injecting state for: {}", plugin.name());
            base = plugin.extend_router(base);
        }

        base
    }

    /// Run plugin shutdown hooks in reverse order.
    async fn shutdown(&self) {
        info!("🛑 {} shutting down", self.name);
        for plugin in self.plugins.iter().rev() {
            info!("🔌 Stopping plugin: {}", plugin.name());
            plugin.on_shutdown().await;
        }
    }

    /// Run the server on plain HTTP.
    pub async fn run(self) {
        let final_router: Router<()> = self.build_router().await;

        info!("🚀 {} starting on port {}", self.name, self.port);
        let listener = TcpListener::bind(&self.address)
            .await
            .expect("Failed to bind server socket");

        serve(listener, final_router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .expect("Server failed to start");

        self.shutdown().await;
    }

    /// Run the server with TLS (HTTPS).
    #[cfg(feature = "tls")]
    pub async fn run_tls(self, cert_pem: &[u8], key_pem: &[u8]) {
        use axum_server::Handle;
        use axum_server::tls_rustls::RustlsConfig;

        let final_router: Router<()> = self.build_router().await;
        let handle = Handle::new();
        let shutdown_handle = handle.clone();

        let config = RustlsConfig::from_pem(cert_pem.to_vec(), key_pem.to_vec())
            .await
            .expect("invalid TLS certificate or key");

        info!("🔒 {} starting on port {} with TLS", self.name, self.port);

        tokio::spawn(async move {
            shutdown_signal().await;
            shutdown_handle.shutdown();
        });

        axum_server::bind_rustls(self.address, config)
            .handle(handle.clone())
            .serve(final_router.into_make_service())
            .await
            .expect("Server failed to start");

        self.shutdown().await;
    }

    /// TLS support is only available when the `tls` feature is enabled.
    #[cfg(not(feature = "tls"))]
    pub async fn run_tls(self, _cert_pem: &[u8], _key_pem: &[u8]) {
        panic!("TLS support requires enabling the `tls` feature");
    }
}
