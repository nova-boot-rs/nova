use crate::{EventEnvelope, MessagingError, NovaMessaging};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct UserCreated {
    id: String,
    email: String,
}

#[tokio::test]
async fn envelope_roundtrip_payload_deserialization() {
    let env = EventEnvelope::new(
        "e-1",
        "users",
        "user.created",
        serde_json::json!({"id":"u1", "email":"u1@nova.rs"}),
    );

    let parsed: UserCreated = env.to_payload().expect("payload should deserialize");
    assert_eq!(parsed.id, "u1");
    assert_eq!(parsed.email, "u1@nova.rs");
}

#[tokio::test]
async fn in_memory_publish_and_poll_json() {
    let messaging = NovaMessaging::in_memory();
    let event = UserCreated {
        id: "u1".to_string(),
        email: "u1@nova.rs".to_string(),
    };

    messaging
        .publish_json("e-1", "users", "user.created", &event)
        .await
        .expect("publish should succeed");

    let out: Vec<UserCreated> = messaging
        .poll_json("users", 10)
        .await
        .expect("poll should succeed");

    assert_eq!(out, vec![event]);
}

#[tokio::test]
async fn failed_handler_routes_to_dlq() {
    let messaging = NovaMessaging::in_memory();
    messaging
        .publish_json(
            "e-1",
            "users",
            "user.created",
            &serde_json::json!({"id":"u1"}),
        )
        .await
        .expect("publish should succeed");

    let processed = messaging
        .process_with_dlq("users", 10, |_env| async {
            Err(MessagingError::Handler("boom".to_string()))
        })
        .await
        .expect("processor should not fail hard");
    assert_eq!(processed, 0);

    let dlq = messaging
        .poll_dlq("users", 10)
        .await
        .expect("dlq poll should succeed");
    assert_eq!(dlq.len(), 1);
    assert_eq!(
        dlq[0].headers.get("x-source-topic"),
        Some(&"users".to_string())
    );
    assert_eq!(
        dlq[0].headers.get("x-dlq-reason"),
        Some(&"handler error: boom".to_string())
    );
    assert_eq!(dlq[0].attempts, 1);
}

#[tokio::test]
async fn kafka_without_server_returns_backend_error() {
    let kafka = NovaMessaging::kafka(vec!["127.0.0.1:65535".to_string()], "nova");

    let e = EventEnvelope::new("e-1", "users", "user.created", serde_json::json!({}));

    let r = kafka.broker.publish(e).await;

    assert!(matches!(r, Err(MessagingError::Backend(_))));
}

#[tokio::test]
async fn nats_without_server_returns_backend_error() {
    let nats = NovaMessaging::nats("nats://127.0.0.1:65535");
    let e = EventEnvelope::new("e-1", "users", "user.created", serde_json::json!({}));
    let r = nats.broker.publish(e).await;
    assert!(matches!(r, Err(MessagingError::Backend(_))));
}

#[tokio::test]
async fn rabbitmq_without_server_returns_backend_error() {
    let rabbit = NovaMessaging::rabbitmq("amqp://127.0.0.1:65535");
    let e = EventEnvelope::new("e-1", "users", "user.created", serde_json::json!({}));
    let r = rabbit.broker.publish(e).await;
    assert!(matches!(r, Err(MessagingError::Backend(_))));
}