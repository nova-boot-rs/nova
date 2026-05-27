use nova_messaging::{InMemoryBroker, EventEnvelope, MessageBroker};

#[tokio::main]
async fn main() {
    let broker = InMemoryBroker::default();

    let envelope = EventEnvelope::new("1", "topic:a", "event.type", serde_json::json!({"hello":"world"}));
    broker.publish(envelope).await.unwrap();

    let msgs = broker.poll("topic:a", 10).await.unwrap();
    println!("polled {} messages", msgs.len());
}
