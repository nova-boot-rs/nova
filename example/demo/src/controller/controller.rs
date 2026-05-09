use nova_core::{Deserialize, Json, axum::Extension, axum::http::StatusCode, get, post, Serialize};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

#[derive(Deserialize)]
pub struct MessageReceived { // Added pub
    pub message: String,
}

// And this
#[derive(Serialize)]
pub struct MessageSent { // Added pub
    pub reply: String,
}

#[get("/hello")]
pub async fn hello_world() -> &'static str {
    "Hello from Nova!\n"
}

#[post("/echo")]
pub async fn echo(Json(payload): Json<MessageReceived>) -> Json<MessageSent> {
    Json(MessageSent {
        reply: format!("Nova received: {}", payload.message),
    })
}

#[get("/users")]
pub async fn get_users(
    Extension(db): Extension<DatabaseConnection>,
) -> Json<Vec<serde_json::Value>> {
    // 1. Explicitly type 'rows' to help the compiler
    let rows: Vec<sea_orm::QueryResult> = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT username, email FROM users", // Updated to match your Entity
            [],
        ))
        .await
        .expect("Failed to query database");

    let mut results = Vec::new();

    for row in rows {
        // 2. Help try_get with types if it fails to infer
        let username: String = row.try_get_by_index(0).unwrap_or_default();
        let email: String = row.try_get_by_index(1).unwrap_or_default();

        results.push(serde_json::json!({
            "username": username,
            "email": email,
        }));
    }

    Json(results)
}

#[get("/db-status")]
pub async fn check_db(Extension(db): Extension<DatabaseConnection>) -> String {
    // Accessing db here tells the compiler the field IS used!
    let backend = db.get_database_backend();
    format!("Nova is connected to: {:?}\n", backend)
}

#[get("/check")]
pub async fn check_something() -> (StatusCode, String) {
    if true {
        (StatusCode::OK, "Everything is fine\n".to_string())
    } else {
        (
            StatusCode::BAD_REQUEST,
            "Something went wrong\n".to_string(),
        )
    }
}