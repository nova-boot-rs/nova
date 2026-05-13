pub use async_trait::async_trait;
pub use axum;
pub use axum::Json;
pub use inventory;
pub use serde::{Deserialize, Serialize};

pub mod config;
pub mod distributed;
pub mod observability;
pub mod resilience;
pub mod runtime;
pub mod traits;

// Error handling and response modules
pub mod error;
pub mod response;

pub use config::{
    EnvConfigSource, JsonFileConfigSource, NovaConfig, NovaConfigBuilder, NovaConfigSource,
    NovaSecretSource,
};
pub use distributed::DistributedStore;
#[cfg(feature = "redis-store")]
pub use distributed::redis_store::RedisStore;
pub use error::{ErrorResponse, NovaError, NovaResult};
pub use observability::{
    NovaMetricsRecorder, ObservabilityConfig, RequestContext, RequestId, init_tracing,
};
pub use resilience::{Bulkhead, CircuitBreaker, RateLimiter, RetryPolicy};
pub use resilience::{DistributedCircuitBreaker, DistributedRateLimiter};
pub use response::{ApiResponse, EmptyResponse, ListResponse};
pub use runtime::{NovaApp, NovaRoute};
pub use traits::{
    NovaLifecycle, NovaModule, NovaPlugin, NovaRequestModel, NovaResponseModel, NovaRouterExtender,
};

extern crate nova_macros;
pub use nova_macros::{NovaRequest, NovaResponse, delete, get, patch, post, put, rest_controller};
