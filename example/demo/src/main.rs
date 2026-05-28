use nova_boot::{NovaApp, spawn_json_file_hot_reloader};
use nova_boot_sql::*;
use std::time::Duration;

mod app_state;
mod dtos;
mod entities;
mod handlers;
mod repositories;
mod services;

use app_state::{AppState, RuntimeConfig};

#[tokio::main]
async fn main() {
    // The demo boots a standard Nova app, then layers on hot-reloaded config,
    // SQL access, auto-registered entities, and the HTTP handlers below.
    let name: &'static str = "NovaApp";
    let port: u16 = 8080; // You can change this to any port you like

    // Runtime settings are loaded from JSON and refreshed every few seconds.
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config/runtime.json");
    let runtime_config = spawn_json_file_hot_reloader(
        config_path,
        Some(RuntimeConfig::default()),
        Some("NOVA_DEMO".to_string()),
        Duration::from_secs(2),
    )
    .expect("failed to start config hot reloader");

    let app_state = AppState { runtime_config };

    // `NovaSql` connects to SQLite, creates the schema from registered entities,
    // and exposes the pool through the `NovaDb` extractor used by handlers.
    let sql_plugin = NovaSql::connect("sqlite:people.db?mode=rwc", true)
        .await
        .add_entity::<entities::user::Entity>()
        .add_entity::<entities::task::Entity>();

    // Optional read replicas come from the live runtime config.
    let config = app_state.runtime_config.get().await;
    if !config.replicas.is_empty() {
        for url in config.replicas.iter() {
            if let Err(e) = sql_plugin.add_replica_url(url).await {
                eprintln!("Failed to add replica {}: {}", url, e);
            }
        }
    }

    // Handlers are discovered through the `#[get]` / `#[post]` macros and
    // registered automatically when the app starts.
    NovaApp::new(name, port, app_state)
        .add_plugin(sql_plugin)
        .run()
        .await;
}
