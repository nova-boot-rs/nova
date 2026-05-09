use axum::routing::MethodRouter;
use axum::{ Router, serve };
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::fmt::init;
pub use inventory;
pub use axum;
pub use axum::Json; 
pub use serde::{Deserialize, Serialize};

extern crate nova_macros;
pub use nova_macros::{rest_controller, get, post, put, delete, patch};

pub struct NovaApp<S=()> where 
    S: Clone + Send + Sync + 'static {
    router: Router<S>,
    address: std::net::SocketAddr,
    state: S, // For future state management
}

pub struct NovaRoute {
    pub path: &'static str,
    pub method: &'static str,
    pub handler: fn() -> MethodRouter<()>,
}

// This allows the inventory crate to collect NovaRoute instances
inventory::collect!(NovaRoute);

impl <S> NovaApp<S> where
    S: Clone + Send + Sync + 'static {
    pub fn new(port: u16, state: S) -> Self {
        let router = Router::<S>::new().layer(TraceLayer::new_for_http()); // Auto-logging for every request!

        Self {
            router,
            address: format!("0.0.0.0:{}", port).parse().expect("Invalid address"),
            state,
        }
    }

    // A "Spring-like" method to add controllers
    pub fn add_route(mut self, path: &str, method_router: MethodRouter<S>) -> Self {
        self.router = self.router.route(path, method_router);
        self
    }

    pub async fn run(self) {
        // Initialize logging automatically (The "Boot" way)
        init();

        let mut app_router = self.router.with_state(self.state.clone());

        for route in inventory::iter::<NovaRoute> {
            info!("📡 Registering {} route: {}", route.method, route.path);
            let method_router = (route.handler)();
            app_router = app_router.route(route.path, method_router);
        }

        let final_router = app_router
        .layer(axum::Extension(self.state.clone())); // Add logging to the final router

        info!("🚀 Nova-Boot starting on {}", self.address);
        let listener = TcpListener::bind(&self.address).await.unwrap();

        serve(listener, final_router).await.expect("Server failed to start");
    }
}
