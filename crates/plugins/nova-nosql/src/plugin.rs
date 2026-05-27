use crate::wrapper::NovaNoSql;
use nova_core::{NovaPlugin, async_trait as nova_async_trait, axum::Extension, axum::Router};

/// Plugin wiring for NoSQL support.
///
/// `NovaNoSql` implements `NovaPlugin` so it can be registered with
/// `NovaApp::add_plugin(...)`. The plugin simply injects a cloned
/// `NovaNoSql` wrapper into request extensions so handlers can extract
/// `NovaDocs`.
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
