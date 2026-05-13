pub use async_trait::async_trait;
pub use axum;
pub use axum::Json;
pub use inventory;
pub use serde::{Deserialize, Serialize};

pub mod config;
pub mod openapi;
pub mod resilience;
pub mod runtime;
pub mod traits;
pub mod validation;

// Error handling and response modules
pub mod error;
pub mod response;

pub use config::{
    EnvConfigSource, JsonFileConfigSource, NovaConfig, NovaConfigBuilder, NovaConfigSource,
    NovaSecretSource, ReloadableConfig, spawn_json_file_hot_reloader,
};
pub use nova_discovery as discovery;
pub use nova_discovery::DistributedStore;
#[cfg(feature = "redis-store")]
pub use nova_discovery::redis_store::RedisStore;
pub use error::{ErrorResponse, NovaError, NovaResult};
pub use nova_observability as observability;
pub use nova_observability::{
    NovaMetricsRecorder, ObservabilityConfig, RequestContext, RequestId, init_tracing,
};
pub use openapi::{OpenApiHook, build_openapi_document};
pub use resilience::{Bulkhead, CircuitBreaker, RateLimiter, RetryPolicy};
pub use resilience::{DistributedCircuitBreaker, DistributedRateLimiter};
pub use response::{
    ApiResponse, ApiVersion, EmptyResponse, ListResponse, PaginatedResponse, PaginationQuery,
    VersionedResponse,
};
pub use runtime::{NovaApp, NovaRoute};
pub use traits::{
    NovaLifecycle, NovaModule, NovaPlugin, NovaRequestModel, NovaResponseModel, NovaRouterExtender,
};
pub use validation::{
    NovaValidate, ValidationErrors, max_length, min_length, required_string, validate_request,
};

extern crate nova_macros;
pub use nova_macros::{NovaRequest, NovaResponse, delete, get, patch, post, put, rest_controller};
