use axum::Router;

#[async_trait::async_trait]
pub trait NovaPlugin: Send + Sync {
    fn name(&self) -> &'static str;

    async fn on_init(&self);

    fn extend_router(&self, router: Router) -> Router;
}

pub trait NovaModule {
    fn module_name(&self) -> &'static str;
}

#[async_trait::async_trait]
pub trait NovaLifecycle: Send + Sync {
    async fn on_start(&self) {}

    async fn on_stop(&self) {}
}

pub trait NovaRouterExtender {
    fn extend_router(&self, router: Router) -> Router;
}
