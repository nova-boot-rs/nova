use crate::NovaGraphDb;
use nova_core::{NovaPlugin, async_trait as nova_async_trait, axum::Extension, axum::Router};

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
