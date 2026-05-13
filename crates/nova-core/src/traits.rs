use axum::Router;

#[async_trait::async_trait]
pub trait NovaPlugin: Send + Sync {
    fn name(&self) -> &'static str;

    async fn on_init(&self);

    async fn on_shutdown(&self) {}

    fn extend_router(&self, router: Router) -> Router;
}

pub trait NovaModule {
    fn module_name(&self) -> &'static str;
}

pub trait NovaRequestModel {
    fn request_model_name() -> &'static str
    where
        Self: Sized,
    {
        std::any::type_name::<Self>()
    }
}

pub trait NovaResponseModel {
    fn response_model_name() -> &'static str
    where
        Self: Sized,
    {
        std::any::type_name::<Self>()
    }
}

#[async_trait::async_trait]
pub trait NovaLifecycle: Send + Sync {
    async fn on_start(&self) {}

    async fn on_stop(&self) {}
}

pub trait NovaRouterExtender {
    fn extend_router(&self, router: Router) -> Router;
}
