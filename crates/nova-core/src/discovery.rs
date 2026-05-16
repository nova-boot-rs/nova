use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use crate::error::DiscoveryError;

/// Represents a discovered service instance.
#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub id: String,
    pub name: String,
    pub address: String,
    pub metadata: HashMap<String, String>,
    pub status: InstanceStatus,
    pub last_heartbeat: Option<std::time::Instant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceStatus {
    Healthy,
    Unhealthy,
    Starting,
    Draining,
}

/// A stream of service instance updates.
pub struct WatchStream {
    pub rx: tokio::sync::mpsc::Receiver<Vec<ServiceInstance>>,
}

#[async_trait]
pub trait Discovery: Send + Sync + 'static {
    /// Register a service instance. Returns error if registration fails.
    async fn register(&self, instance: ServiceInstance) -> Result<(), DiscoveryError>;

    /// Discover all healthy instances of a service.
    async fn discover(&self, service_name: &str) -> Result<Vec<ServiceInstance>, DiscoveryError>;

    /// Send a heartbeat to keep registration alive.
    async fn heartbeat(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Result<(), DiscoveryError>;

    /// Remove a service instance from the registry.
    async fn deregister(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Result<(), DiscoveryError>;

    /// Watch for changes to a service's instances. Returns a stream of instance lists.
    async fn watch(&self, service_name: &str) -> Result<WatchStream, DiscoveryError>;
}