use crate::NovaGraphDb;
use nova_boot::{NovaPlugin, async_trait as nova_async_trait, axum::Extension, axum::Router};

/// Plugin wiring for graph database support.
///
/// `NovaGraphDb` implements `NovaPlugin` so it can be registered with
/// `NovaApp`. The plugin injects a cloned `NovaGraphDb` into request
/// extensions to enable the `NovaGraph` extractor in handlers.
#[nova_async_trait]
impl NovaPlugin for NovaGraphDb {
    fn name(&self) -> &'static str {
        "NovaGraphDb"
    }

    async fn on_init(&self) {
        println!("🕸️ Initializing GraphDB Plugin...");
    }

    fn extend_router(&self, router: Router<()>) -> Router<()> {
        router.layer(Extension(self.clone()))
    }
}
