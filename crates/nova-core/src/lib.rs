pub use async_trait::async_trait;
pub use axum;
pub use axum::Json;
pub use inventory;
pub use serde::{Deserialize, Serialize};

pub mod config;
pub mod error;
pub mod runtime;
pub mod traits;

// Error handling and response modules
pub use config::{
    EnvConfigSource, JsonFileConfigSource, NovaConfig, NovaConfigBuilder, NovaConfigSource,
    NovaSecretSource, ReloadableConfig, spawn_json_file_hot_reloader,
};
pub use error::{ErrorResponse, NovaError, NovaResult};
pub use nova_observability as observability;
pub use nova_observability::{
    NovaMetricsRecorder, ObservabilityConfig, RequestContext, RequestId, init_tracing,
};
pub use nova_resilience_store as discovery;
pub use nova_resilience_store::ResilienceStore;
#[cfg(feature = "redis-store")]
pub use nova_resilience_store::redis_store::RedisStore;
pub use runtime::{NovaApp, NovaRoute};
pub use traits::{
    NovaLifecycle, NovaModule, NovaPlugin, NovaRequestModel, NovaResponseModel, NovaRouterExtender,
};

extern crate nova_macros;
pub use nova_macros::{NovaRequest, NovaResponse, delete, get, patch, post, put, rest_controller};
