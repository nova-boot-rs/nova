pub use async_trait::async_trait;
pub use axum;
pub use axum::Json;
pub use inventory;
pub use serde::{Deserialize, Serialize};

pub mod runtime;
pub mod config;
pub mod observability;
pub mod traits;

// Error handling and response modules
pub mod error;
pub mod response;

pub use error::{ErrorResponse, NovaError, NovaResult};
pub use config::{EnvConfigSource, JsonFileConfigSource, NovaConfig, NovaConfigBuilder, NovaConfigSource, NovaSecretSource};
pub use observability::{init_tracing, NovaMetricsRecorder, ObservabilityConfig, RequestContext, RequestId};
pub use response::{ApiResponse, EmptyResponse, ListResponse};
pub use runtime::{NovaApp, NovaRoute};
pub use traits::{NovaLifecycle, NovaModule, NovaPlugin, NovaRouterExtender};

extern crate nova_macros;
pub use nova_macros::{delete, get, patch, post, put, rest_controller};
