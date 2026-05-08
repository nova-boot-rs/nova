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
pub use nova_macros::rest_controller;
pub use nova_macros::get;
pub use nova_macros::post;

pub struct NovaApp {
    router: axum::Router,
    address: std::net::SocketAddr,
}

pub struct NovaRoute {
    pub path: &'static str,
    pub method: &'static str,
    pub handler: fn() -> MethodRouter,
}

// This allows the inventory crate to collect NovaRoute instances
inventory::collect!(NovaRoute);

impl NovaApp {
    pub fn new(port: u16) -> Self {
        let router = Router::new().layer(TraceLayer::new_for_http()); // Auto-logging for every request!

        Self {
            router,
            address: format!("0.0.0.0:{}", port).parse().expect("Invalid address"),
        }
    }

    // A "Spring-like" method to add controllers
    pub fn add_route(mut self, path: &str, method_router: MethodRouter) -> Self {
        self.router = self.router.route(path, method_router);
        self
    }

    pub async fn run(mut self) {
        // Initialize logging automatically (The "Boot" way)
        init();

        for route in inventory::iter::<NovaRoute> {
            info!("📡 Registering {} route: {}", route.method, route.path);
            let method_router = (route.handler)();
            self.router = self.router.route(route.path, method_router);
        }

        info!("🚀 Nova-Boot starting on {}", self.address);
        let listener = TcpListener::bind(&self.address).await.unwrap();
        serve(listener, self.router).await.unwrap();
    }
}
