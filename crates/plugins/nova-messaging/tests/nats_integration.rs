use nova_messaging::{EventEnvelope, NovaMessaging};
use std::time::Duration;

#[tokio::test]
async fn nats_publish_poll_and_dlq_roundtrip() {
    let url = match std::env::var("NATS_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Skipping NATS integration test: NATS_URL is not set");
            return;
        }
    };

    let messaging = NovaMessaging::nats(url);

    let topic = format!("nova.test.{}", uuid_like_suffix());
    let dlq_source = format!("nova.dlq.{}", uuid_like_suffix());

    let envelope = EventEnvelope::new(
        format!("e-{}", uuid_like_suffix()),
        topic.clone(),
        "user.created",
        serde_json::json!({"id":"u1"}),
    );

    messaging
        .poll(&topic, 1)
        .await
        .expect("initial poll should work even when empty");

    messaging
        .poll_dlq(&dlq_source, 1)
        .await
        .expect("initial dlq poll should work even when empty");

    messaging
        .publish_envelope(envelope.clone())
        .await
        .expect("publish should succeed");

    let messages = messaging
        .poll(&topic, 10)
        .await
        .expect("poll should succeed");
    assert!(
        !messages.is_empty(),
        "expected at least one message from NATS subject"
    );

    messaging
        .publish_to_dlq(&dlq_source, envelope, "handler failed")
        .await
        .expect("publish dlq should succeed");

    let mut dlq = Vec::new();
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(200)).await;
        dlq = messaging
            .poll_dlq(&dlq_source, 10)
            .await
            .expect("dlq poll should succeed");
        if !dlq.is_empty() {
            break;
        }
    }

    assert!(!dlq.is_empty(), "expected at least one dlq message");
    assert_eq!(dlq[0].headers.get("x-source-topic"), Some(&dlq_source));
}

fn uuid_like_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
