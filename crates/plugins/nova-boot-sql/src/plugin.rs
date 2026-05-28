use crate::{connection::NovaSql, pool::ReadWritePool, tenant_middleware::tenant_middleware};
use nova_boot::{NovaPlugin, async_trait, axum::Extension, axum::Router, axum::middleware};
use std::sync::Arc;

/// Plugin integration for Nova applications.
///
/// Implements `NovaPlugin` for `NovaSql` so the plugin can be added to the
/// application via `NovaApp::add_plugin(...)`. During `extend_router` the
/// plugin injects a `ReadWritePool` into request extensions so handlers can
/// extract `NovaDb`.
#[async_trait]
impl NovaPlugin for NovaSql {
    fn name(&self) -> &'static str {
        "NovaSql (Relational Engine)"
    }

    async fn on_init(&self) {
        println!("🗄️ Initializing SQL Plugin...");
        for task in &self.sync_tasks {
            task.run(self).await;
        }
    }

    fn extend_router(&self, mut router: Router<()>) -> Router<()> {
        let pool: ReadWritePool = self.read_write_pool();
        router = router.layer(Extension(pool));

        if let Some(resolver) = &self.tenant_resolver {
            router = router.layer(middleware::from_fn_with_state(
                Arc::clone(resolver),
                tenant_middleware,
            ));
        }
        router
    }
}
