use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::Value;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn debug_mode_includes_details() {
    // SAFETY: Tests run serially via #[serial], so no other thread
    // accesses environment variables concurrently
    unsafe {
        std::env::set_var("NOVA_DEBUG", "1");
    }

    let err = nova_boot::NovaError::InternalError("boom".into());
    let resp = err.into_response();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let bytes = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("body bytes");
    let v: Value = serde_json::from_slice(&bytes).expect("valid json");

    assert!(
        v.get("details").and_then(|d| d.as_str()).is_some(),
        "details should be present when NOVA_DEBUG=1"
    );

    // SAFETY: Still running serially, safe to clean up
    unsafe {
        std::env::remove_var("NOVA_DEBUG");
    }
}
