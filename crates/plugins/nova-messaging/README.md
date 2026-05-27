# `nova-messaging`

Purpose

- Message broker integrations: Kafka, RabbitMQ, NATS; consumer helpers and DLQ support.

Quick start

```rust
use nova_messaging::NovaMessaging;
let bus = NovaMessaging::in_memory();
NovaApp::new("svc", 8080, state).add_plugin(bus).run().await;
```

Highlights

- Consumers with DLQ support and retry helpers.
- In-memory broker for development and tests.

Docs & examples

- See `crates/plugins/nova-messaging/src` and `example/` for consumer/producer patterns.
