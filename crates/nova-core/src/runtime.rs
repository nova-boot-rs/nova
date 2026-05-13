use crate::traits::NovaPlugin;
use axum::routing::MethodRouter;
use axum::{Router, serve};
use nova_observability::{init_tracing, request_id_layer};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

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

        let mut final_router = app_router.layer(axum::Extension(self.state.clone()));

        for plugin in &self.plugins {
            info!("🔌 Injecting state for: {}", plugin.name());
            final_router = plugin.extend_router(final_router);
        }

        info!("🚀 {{{}}} starting on port {}", self.name, self.port);
        let listener = TcpListener::bind(&self.address)
            .await
            .expect("Failed to bind server socket");

        serve(listener, final_router)
            .await
            .expect("Server failed to start");
    }
}
