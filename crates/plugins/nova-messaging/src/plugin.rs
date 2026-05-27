use crate::NovaMessaging;
use nova_core::{NovaPlugin, async_trait as nova_async_trait, axum::Extension, axum::Router};

/// Messaging plugin wiring.
///
/// Registers `NovaMessaging` as a `NovaPlugin` so applications can add the
/// messaging subsystem via `NovaApp::add_plugin(...)`. The plugin injects a
/// cloned `NovaMessaging` wrapper into request extensions enabling the
/// `NovaBus` extractor for handlers.
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
