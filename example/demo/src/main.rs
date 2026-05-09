use nova_core::NovaApp;
use nova_sql::*;

mod controller;
mod entity;

#[tokio::main]
async fn main() {
    let port: u16 = 8080; // You can change this to any port you like
    let sql_plugin = NovaSql::connect("sqlite:people.db?mode=rwc", true)
        .await
        .add_entity::<entity::user::Entity>();
    // No manual routing needed! run() finds hello_world automatically.
    NovaApp::new(port, ()).add_plugin(sql_plugin).run().await;
}
