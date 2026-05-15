use nova_core::{NovaApp, spawn_json_file_hot_reloader};
use nova_sql::*;
use std::time::Duration;

mod app_state;
mod controller;
mod entity;

use app_state::{AppState, RuntimeConfig};

#[tokio::main]
async fn main() {
    let name: &'static str = "NovaApp";
    let port: u16 = 8080; // You can change this to any port you like
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config/runtime.json");
    let runtime_config = spawn_json_file_hot_reloader(
        config_path,
        Some(RuntimeConfig::default()),
        Some("NOVA_DEMO".to_string()),
        Duration::from_secs(2),
    )
    .expect("failed to start config hot reloader");

    let app_state = AppState { runtime_config };

    let sql_plugin = NovaSql::connect("sqlite:people.db?mode=rwc", true)
        .await
        .add_entity::<entity::user::Entity>();
    // Register replicas from runtime config if provided
    let config = app_state.runtime_config.get().await;
    if !config.replicas.is_empty() {
        for url in config.replicas.iter() {
            if let Err(e) = sql_plugin.add_replica_url(url).await {
                eprintln!("Failed to add replica {}: {}", url, e);
            }
        }
    }
    // No manual routing needed! run() finds hello_world automatically.
    NovaApp::new(name, port, app_state)
        .add_plugin(sql_plugin)
        .run()
        .await;
}
