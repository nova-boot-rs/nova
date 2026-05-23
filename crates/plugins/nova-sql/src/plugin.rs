use crate::{pool::ReadWritePool, tenant_middleware::tenant_middleware, connection::NovaSql};
use nova_core::{
    async_trait, axum::middleware, axum::Extension, axum::Router, NovaPlugin,
};
use std::sync::Arc;

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