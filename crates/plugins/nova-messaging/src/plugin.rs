use crate::NovaMessaging;
use nova_core::{axum::Extension, axum::Router, async_trait as nova_async_trait, NovaPlugin};

#[nova_async_trait]
impl NovaPlugin for NovaMessaging {
    fn name(&self) -> &'static str {
        "NovaMessaging"
    }

    async fn on_init(&self) {
        tracing::info!("📨 Initializing Messaging Plugin...");
    }

    fn extend_router(&self, router: Router<()>) -> Router<()> {
        router.layer(Extension(self.clone()))
    }
}