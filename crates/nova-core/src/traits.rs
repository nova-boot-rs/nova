use axum::Router;

#[async_trait::async_trait]
/// A plugin provides optional features to a Nova application.
///
/// Plugins should implement lifecycle hooks and may extend the application's
/// router to register additional routes or middleware. Plugins are loaded and
/// initialized by `NovaApp` during startup.
pub trait NovaPlugin: Send + Sync {
    /// Human-readable plugin name used in logs.
    fn name(&self) -> &'static str;

    /// Initialization hook called during application startup.
    async fn on_init(&self);

    /// Shutdown hook called during application shutdown. Default is noop.
    async fn on_shutdown(&self) {}

    /// Extend the provided `Router<()>` with plugin routes or layers.
    fn extend_router(&self, router: Router<()>) -> Router<()>;
}

/// Represents a logical module within the framework.
pub trait NovaModule {
    fn module_name(&self) -> &'static str;
}

/// Marker trait for request models. Implementations can override
/// `request_model_name` to provide a readable type name for docs and telemetry.
pub trait NovaRequestModel {
    fn request_model_name() -> &'static str
    where
        Self: Sized,
    {
        std::any::type_name::<Self>()
    }
}

/// Marker trait for response models. Implementations can override
/// `response_model_name` to provide a readable type name for docs and telemetry.
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
