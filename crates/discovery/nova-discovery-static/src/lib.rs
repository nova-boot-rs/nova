use async_trait::async_trait;
use nova_core::discovery::{Discovery, DiscoveryError, InstanceStatus, ServiceInstance, WatchStream};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct StaticDiscovery {
    services: RwLock<HashMap<String, Vec<ServiceInstance>>>,
    watchers: RwLock<HashMap<String, Vec<tokio::sync::mpsc::Sender<Vec<ServiceInstance>>>>>,
}

impl StaticDiscovery {
    /// Create from a list of service instances.
    pub fn new(instances: Vec<ServiceInstance>) -> Self {
        let mut services: HashMap<String, Vec<ServiceInstance>> = HashMap::new();
        for instance in instances {
            services
                .entry(instance.name.clone())
                .or_default()
                .push(Self::normalize_instance(instance));
        }

        Self {
            services: RwLock::new(services),
            watchers: RwLock::new(HashMap::new()),
        }
    }

    /// Create from a simple mapping: service_name -> ["host:port", ...]
    pub fn from_map(map: HashMap<String, Vec<String>>) -> Self {
        let mut services: HashMap<String, Vec<ServiceInstance>> = HashMap::new();

        for (service_name, addresses) in map {
            let instances = addresses
                .into_iter()
                .map(|address| Self::instance_from_address(&service_name, address))
                .collect();
            services.insert(service_name, instances);
        }

        Self {
            services: RwLock::new(services),
            watchers: RwLock::new(HashMap::new()),
        }
    }

    /// Add a single instance programmatically.
    pub async fn add_instance(&self, instance: ServiceInstance) {
        let _ = self.register(instance).await;
    }

    fn normalize_instance(mut instance: ServiceInstance) -> ServiceInstance {
        instance.status = InstanceStatus::Healthy;
        instance.last_heartbeat = None;
        instance
    }

    fn instance_from_address(service_name: &str, address: String) -> ServiceInstance {
        ServiceInstance {
            id: address.clone(),
            name: service_name.to_string(),
            address,
            metadata: HashMap::new(),
            status: InstanceStatus::Healthy,
            last_heartbeat: None,
        }
    }

    async fn notify_watchers(&self, service_name: &str) {
        let snapshot = match self.discover(service_name).await {
            Ok(instances) => instances,
            Err(_) => return,
        };

        let watchers = {
            let watchers = self.watchers.read().await;
            watchers.get(service_name).cloned().unwrap_or_default()
        };

        for watcher in watchers {
            let _ = watcher.send(snapshot.clone()).await;
        }
    }
}

#[async_trait]
impl Discovery for StaticDiscovery {
    async fn register(&self, instance: ServiceInstance) -> Result<(), DiscoveryError> {
        let instance = Self::normalize_instance(instance);
        let service_name = instance.name.clone();

        {
            let mut services = self.services.write().await;
            let service_instances = services.entry(service_name.clone()).or_default();
            service_instances.retain(|existing| existing.id != instance.id);
            service_instances.push(instance);
        }

        self.notify_watchers(&service_name).await;
        Ok(())
    }

    async fn discover(&self, service_name: &str) -> Result<Vec<ServiceInstance>, DiscoveryError> {
        let services = self.services.read().await;
        let instances = services.get(service_name).cloned().unwrap_or_default();
        Ok(instances
            .into_iter()
            .filter(|instance| matches!(instance.status, InstanceStatus::Healthy))
            .collect())
    }

    async fn heartbeat(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Result<(), DiscoveryError> {
        let mut found = false;
        {
            let mut services = self.services.write().await;
            let instances = services
                .get_mut(service_name)
                .ok_or_else(|| DiscoveryError::NotFound(service_name.to_string()))?;

            for instance in instances.iter_mut() {
                if instance.id == instance_id {
                    instance.status = InstanceStatus::Healthy;
                    instance.last_heartbeat = Some(std::time::Instant::now());
                    found = true;
                    break;
                }
            }
        }

        if !found {
            return Err(DiscoveryError::NotFound(format!(
                "service '{}' instance '{}' not found",
                service_name, instance_id
            )));
        }

        self.notify_watchers(service_name).await;
        Ok(())
    }

    async fn deregister(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Result<(), DiscoveryError> {
        let removed = {
            let mut services = self.services.write().await;
            let instances = services
                .get_mut(service_name)
                .ok_or_else(|| DiscoveryError::NotFound(service_name.to_string()))?;
            let before = instances.len();
            instances.retain(|instance| instance.id != instance_id);
            let removed = instances.len() != before;
            if instances.is_empty() {
                services.remove(service_name);
            }
            removed
        };

        if !removed {
            return Err(DiscoveryError::NotFound(format!(
                "service '{}' instance '{}' not found",
                service_name, instance_id
            )));
        }

        self.notify_watchers(service_name).await;
        Ok(())
    }

    async fn watch(&self, service_name: &str) -> Result<WatchStream, DiscoveryError> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        {
            let mut watchers = self.watchers.write().await;
            watchers
                .entry(service_name.to_string())
                .or_default()
                .push(tx.clone());
        }

        let initial_snapshot = self.discover(service_name).await?;
        let _ = tx.send(initial_snapshot).await;

        Ok(WatchStream { rx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_instance(service: &str, id: &str, address: &str) -> ServiceInstance {
        ServiceInstance {
            id: id.to_string(),
            name: service.to_string(),
            address: address.to_string(),
            metadata: HashMap::new(),
            status: InstanceStatus::Healthy,
            last_heartbeat: None,
        }
    }

    #[tokio::test]
    async fn register_and_discover() {
        let discovery = StaticDiscovery::new(vec![]);
        let instance = test_instance("users", "users-1", "127.0.0.1:9000");

        discovery.register(instance.clone()).await.expect("register should succeed");

        let instances = discovery.discover("users").await.expect("discover should succeed");
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].id, instance.id);
        assert_eq!(instances[0].address, instance.address);
        assert_eq!(instances[0].status, InstanceStatus::Healthy);
    }

    #[tokio::test]
    async fn deregister_and_discover_empty() {
        let discovery = StaticDiscovery::new(vec![test_instance("users", "users-1", "127.0.0.1:9000")]);

        discovery
            .deregister("users", "users-1")
            .await
            .expect("deregister should succeed");

        let instances = discovery.discover("users").await.expect("discover should succeed");
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn heartbeat_updates_timestamp() {
        let discovery = StaticDiscovery::new(vec![test_instance("users", "users-1", "127.0.0.1:9000")]);

        let before = discovery
            .discover("users")
            .await
            .expect("discover should succeed")[0]
            .last_heartbeat;
        assert!(before.is_none());

        discovery
            .heartbeat("users", "users-1")
            .await
            .expect("heartbeat should succeed");

        let after = discovery
            .discover("users")
            .await
            .expect("discover should succeed")[0]
            .last_heartbeat;
        assert!(after.is_some());
    }
}
