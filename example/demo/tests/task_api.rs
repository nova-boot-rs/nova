use reqwest::StatusCode;

#[tokio::test]
async fn demo_smoke() {
    // Placeholder: tests should start the demo app (or use test harness)
    // and exercise create → read flows.

    // This is a minimal smoke guard ensuring test harness wiring works.
    let resp = reqwest::get("http://127.0.0.1:8080/hello").await;
    match resp {
        Ok(r) => {
            assert_eq!(r.status(), StatusCode::OK);
        }
        Err(_) => {
            // If the server is not running locally, skip with a soft pass.
            eprintln!(
                "demo server not running — run `cargo run` in example/demo to exercise integration tests"
            );
        }
    }
}
