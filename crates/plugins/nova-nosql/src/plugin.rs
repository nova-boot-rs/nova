use crate::wrapper::NovaNoSql;
use nova_core::{NovaPlugin, async_trait as nova_async_trait, axum::Extension, axum::Router};

#[nova_async_trait]
impl NovaPlugin for NovaNoSql {
    fn name(&self) -> &'static str {
        "NovaNoSql"
    }

    async fn on_init(&self) {
        println!("🧩 Initializing NoSQL Plugin...");
    }

    fn extend_router(&self, router: Router<()>) -> Router<()> {
        router.layer(Extension(self.clone()))
    }
}
