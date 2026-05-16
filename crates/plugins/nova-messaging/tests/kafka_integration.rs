use nova_messaging::{EventEnvelope, NovaMessaging};

#[tokio::test]
async fn kafka_publish_poll_and_dlq_roundtrip() {
    let brokers = match std::env::var("NOVA_TEST_KAFKA_BROKERS") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Skipping Kafka integration test: NOVA_TEST_KAFKA_BROKERS is not set");
            return;
        }
    };

    let messaging = NovaMessaging::kafka(
        brokers.split(',').map(|s| s.trim().to_string()).collect(),
        "nova-test",
    );

    let topic = format!("nova.test.{}", nanos_suffix());
    let dlq_source = format!("nova.dlq.{}", nanos_suffix());

    let envelope = EventEnvelope::new(
        format!("e-{}", nanos_suffix()),
        topic.clone(),
        "user.created",
        serde_json::json!({"id":"u1"}),
    );

    messaging
        .publish_envelope(envelope.clone())
        .await
        .expect("publish should succeed");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let messages = messaging
        .poll(&topic, 10)
        .await
        .expect("poll should succeed");
    assert!(
        !messages.is_empty(),
        "expected at least one message from Kafka topic"
    );
    assert_eq!(messages[0].id, envelope.id);

    messaging
        .publish_to_dlq(&dlq_source, envelope, "handler failed")
        .await
        .expect("publish dlq should succeed");

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let dlq = messaging
        .poll_dlq(&dlq_source, 10)
        .await
        .expect("dlq poll should succeed");

    assert!(!dlq.is_empty(), "expected at least one dlq message");
    assert_eq!(dlq[0].headers.get("x-source-topic"), Some(&dlq_source));
    assert_eq!(dlq[0].attempts, 1);
}

fn nanos_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
