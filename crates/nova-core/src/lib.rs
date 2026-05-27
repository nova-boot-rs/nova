pub use async_trait::async_trait;
pub use axum;
pub use axum::Json;
pub use inventory;
pub use serde::{Deserialize, Serialize};

pub mod config;
pub mod error;
pub mod runtime;
pub mod state;
pub mod traits;

// Error handling and response modules
pub use config::{
    EnvConfigSource, JsonFileConfigSource, NovaConfig, NovaConfigBuilder, NovaConfigSource,
    NovaSecretSource, ReloadableConfig, spawn_json_file_hot_reloader,
};
pub use error::{ErrorResponse, NovaError, NovaResult};
pub mod discovery;

pub use discovery::{Discovery, InstanceStatus, ServiceInstance};
pub use error::DiscoveryError;
// Observability is implemented by an optional plugin crate (nova-observability).
pub use nova_resilience_store::LuaValue;
pub use nova_resilience_store::ResilienceStore;
#[cfg(feature = "redis-store")]
pub use nova_resilience_store::redis_store::RedisStore;
pub use runtime::{NovaApp, NovaRoute};
pub use traits::{
    NovaLifecycle, NovaModule, NovaPlugin, NovaRequestModel, NovaResponseModel, NovaRouterExtender,
};

pub use state::NovaState;
// Public documentation: core types are documented in their respective modules
// (see `crates/nova-core/src/runtime.rs`, `state.rs`, and `error.rs`).

// Observability plugin lives in `crates/nova-observability` to avoid cycles.

extern crate nova_macros;
pub use nova_macros::{NovaRequest, NovaResponse, delete, get, patch, post, put, rest_controller};
