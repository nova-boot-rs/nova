use nova_core::{NovaApp, get, post, Json};
use serde::Deserialize;

#[get("/hello")]
async fn hello_world() -> &'static str {
    "Hello from Nova!"
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[post("/echo")]
async fn echo(Json(payload): Json<Message>) -> String {
    format!("Nova received: {}", payload.content)
}

#[tokio::main]
async fn main() {
    let port: u16 = 8080; // You can change this to any port you like
    // No manual routing needed! run() finds hello_world automatically.
    NovaApp::new(port).run().await;
}
