use async_trait::async_trait;
use etcd_client::{Client, DeleteOptions, EventType, GetOptions, PutOptions, WatchOptions};
use nova_core::discovery::{
    Discovery, DiscoveryError, InstanceStatus, ServiceInstance, WatchStream,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, mpsc};
use tokio::time::Duration;
use tracing::warn;

#[derive(Clone)]
pub struct EtcdDiscovery {
    client: Client,
    prefix: String,
    lease_ttl: i64,
    watchers: Arc<RwLock<HashMap<String, Vec<mpsc::Sender<Vec<ServiceInstance>>>>>>,
    watch_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    leases: Arc<RwLock<HashMap<String, i64>>>,
}

impl EtcdDiscovery {
    pub async fn new(
        endpoints: Vec<String>,
        prefix: impl Into<String>,
        lease_ttl: i64,
    ) -> Result<Self, DiscoveryError> {
        let client = Client::connect(endpoints.as_slice(), None)
            .await
            .map_err(|error| DiscoveryError::ConnectionFailed(error.to_string()))?;

        Ok(Self {
            client,
            prefix: Self::normalize_prefix(prefix.into()),
            lease_ttl,
            watchers: Arc::new(RwLock::new(HashMap::new())),
            watch_tasks: Arc::new(RwLock::new(HashMap::new())),
            leases: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn normalize_prefix(prefix: String) -> String {
        prefix.trim_matches('/').to_string()
    }

    fn join_prefix(prefix: &str, service_name: &str) -> String {
        let prefix = prefix.trim_matches('/');
        if prefix.is_empty() {
            format!("/{service_name}")
        } else {
            format!("/{prefix}/{service_name}")
        }
    }

    fn service_prefix_from(prefix: &str, service_name: &str) -> String {
        format!("{}/", Self::join_prefix(prefix, service_name))
    }

    fn instance_key_from(prefix: &str, service_name: &str, instance_id: &str) -> String {
        format!(
            "{}{}",
            Self::service_prefix_from(prefix, service_name),
            instance_id
        )
    }

    fn service_prefix(&self, service_name: &str) -> String {
        Self::service_prefix_from(&self.prefix, service_name)
    }

    fn instance_key(&self, service_name: &str, instance_id: &str) -> String {
        Self::instance_key_from(&self.prefix, service_name, instance_id)
    }

    fn now_millis() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }

    fn service_instance_to_record(instance: &ServiceInstance) -> StoredServiceInstance {
        StoredServiceInstance {
            id: instance.id.clone(),
            name: instance.name.clone(),
            address: instance.address.clone(),
            metadata: instance.metadata.clone(),
            status: InstanceStatus::Healthy,
            last_heartbeat_unix_ms: Some(Self::now_millis()),
        }
    }

    fn service_instance_from_record(record: StoredServiceInstance) -> ServiceInstance {
        let last_heartbeat = record.last_heartbeat_unix_ms.and_then(|millis| {
            let now = Self::now_millis();
            let delta = now.saturating_sub(millis);
            Instant::now().checked_sub(Duration::from_millis(delta))
        });

        ServiceInstance {
            id: record.id,
            name: record.name,
            address: record.address,
            metadata: record.metadata,
            status: record.status,
            last_heartbeat,
        }
    }

    fn parse_service_instance(value: &[u8]) -> Result<ServiceInstance, DiscoveryError> {
        let record: StoredServiceInstance = serde_json::from_slice(value)
            .map_err(|error| DiscoveryError::Backend(error.to_string()))?;
        Ok(Self::service_instance_from_record(record))
    }

    fn map_backend_error(error: etcd_client::Error) -> DiscoveryError {
        DiscoveryError::Backend(error.to_string())
    }

    async fn discover_instances(
        &self,
        service_name: &str,
    ) -> Result<Vec<ServiceInstance>, DiscoveryError> {
        let mut client = self.client.clone();
        let response = client
            .get(
                self.service_prefix(service_name),
                Some(GetOptions::new().with_prefix()),
            )
            .await
            .map_err(Self::map_backend_error)?;

        let mut instances = Vec::with_capacity(response.kvs().len());
        for kv in response.kvs() {
            let instance = Self::parse_service_instance(kv.value())?;
            if instance.status == InstanceStatus::Healthy {
                instances.push(instance);
            }
        }

        Ok(instances)
    }

    async fn notify_watchers(&self, service_name: &str) {
        let snapshot = match self.discover_instances(service_name).await {
            Ok(instances) => instances,
            Err(error) => {
                warn!(service = %service_name, error = %error, "etcd watcher snapshot failed");
                return;
            }
        };

        let watchers = {
            let watchers = self.watchers.read().await;
            watchers.get(service_name).cloned().unwrap_or_default()
        };

        if watchers.is_empty() {
            return;
        }

        let mut alive = Vec::with_capacity(watchers.len());
        for watcher in watchers {
            if watcher.send(snapshot.clone()).await.is_ok() {
                alive.push(watcher);
            }
        }

        let mut watchers_map = self.watchers.write().await;
        if let Some(entry) = watchers_map.get_mut(service_name) {
            *entry = alive;
        }
    }

    async fn watch_loop(self, service_name: String) {
        loop {
            let has_watchers = {
                let watchers = self.watchers.read().await;
                watchers
                    .get(&service_name)
                    .map(|entries| !entries.is_empty())
                    .unwrap_or(false)
            };

            if !has_watchers {
                break;
            }

            let mut client = self.client.clone();
            let watch_result = client
                .watch(
                    self.service_prefix(&service_name),
                    Some(WatchOptions::new().with_prefix()),
                )
                .await;

            let (_watcher, mut stream) = match watch_result {
                Ok(result) => result,
                Err(error) => {
                    warn!(service = %service_name, error = %error, "etcd watch setup failed");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            loop {
                let response = match stream.message().await {
                    Ok(Some(response)) => response,
                    Ok(None) => break,
                    Err(error) => {
                        warn!(service = %service_name, error = %error, "etcd watch stream failed");
                        break;
                    }
                };

                if response.canceled() {
                    break;
                }

                if response
                    .events()
                    .iter()
                    .any(|event| matches!(event.event_type(), EventType::Put | EventType::Delete))
                {
                    self.notify_watchers(&service_name).await;
                }
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        self.watch_tasks.write().await.remove(&service_name);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredServiceInstance {
    id: String,
    name: String,
    address: String,
    metadata: HashMap<String, String>,
    status: InstanceStatus,
    last_heartbeat_unix_ms: Option<u64>,
}

#[async_trait]
impl Discovery for EtcdDiscovery {
    async fn register(&self, instance: ServiceInstance) -> Result<(), DiscoveryError> {
        let key = self.instance_key(&instance.name, &instance.id);
        let record = Self::service_instance_to_record(&instance);
        let payload = serde_json::to_vec(&record)
            .map_err(|error| DiscoveryError::Backend(error.to_string()))?;

        if let Some(lease_id) = self.leases.read().await.get(&key).copied() {
            let mut client = self.client.clone();
            if let Err(error) = client.lease_revoke(lease_id).await {
                warn!(service = %instance.name, instance = %instance.id, error = %error, "failed to revoke previous etcd lease");
            }
        }

        let mut client = self.client.clone();
        let lease = client
            .lease_grant(self.lease_ttl, None)
            .await
            .map_err(Self::map_backend_error)?;

        client
            .put(
                key.clone(),
                payload,
                Some(PutOptions::new().with_lease(lease.id())),
            )
            .await
            .map_err(Self::map_backend_error)?;

        self.leases.write().await.insert(key, lease.id());
        self.notify_watchers(&instance.name).await;
        Ok(())
    }

    async fn discover(&self, service_name: &str) -> Result<Vec<ServiceInstance>, DiscoveryError> {
        self.discover_instances(service_name).await
    }

    async fn heartbeat(&self, service_name: &str, instance_id: &str) -> Result<(), DiscoveryError> {
        let key = self.instance_key(service_name, instance_id);
        let lease_id = self.leases.read().await.get(&key).copied().ok_or_else(|| {
            DiscoveryError::NotFound(format!(
                "service '{}' instance '{}' not found",
                service_name, instance_id
            ))
        })?;

        let mut client = self.client.clone();
        let (mut keeper, mut lease_stream) = client
            .lease_keep_alive(lease_id)
            .await
            .map_err(Self::map_backend_error)?;
        keeper.keep_alive().await.map_err(Self::map_backend_error)?;
        let _ = lease_stream
            .message()
            .await
            .map_err(Self::map_backend_error)?;

        let response = client
            .get(key.clone(), None)
            .await
            .map_err(Self::map_backend_error)?;

        let kv = response.kvs().first().ok_or_else(|| {
            DiscoveryError::NotFound(format!(
                "service '{}' instance '{}' not found",
                service_name, instance_id
            ))
        })?;

        let mut instance = Self::parse_service_instance(kv.value())?;
        instance.status = InstanceStatus::Healthy;
        instance.last_heartbeat = Some(Instant::now());

        let record = StoredServiceInstance {
            id: instance.id.clone(),
            name: instance.name.clone(),
            address: instance.address.clone(),
            metadata: instance.metadata.clone(),
            status: InstanceStatus::Healthy,
            last_heartbeat_unix_ms: Some(Self::now_millis()),
        };

        let payload = serde_json::to_vec(&record)
            .map_err(|error| DiscoveryError::Backend(error.to_string()))?;

        client
            .put(key, payload, Some(PutOptions::new().with_lease(lease_id)))
            .await
            .map_err(Self::map_backend_error)?;

        self.notify_watchers(service_name).await;
        Ok(())
    }

    async fn deregister(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Result<(), DiscoveryError> {
        let key = self.instance_key(service_name, instance_id);
        let lease_id = self.leases.write().await.remove(&key);

        let mut client = self.client.clone();
        let delete_response = client
            .delete(key.clone(), Some(DeleteOptions::new()))
            .await
            .map_err(Self::map_backend_error)?;

        if delete_response.deleted() == 0 {
            return Err(DiscoveryError::NotFound(format!(
                "service '{}' instance '{}' not found",
                service_name, instance_id
            )));
        }

        if let Some(lease_id) = lease_id {
            let _ = client.lease_revoke(lease_id).await;
        }

        self.notify_watchers(service_name).await;
        Ok(())
    }

    async fn watch(&self, service_name: &str) -> Result<WatchStream, DiscoveryError> {
        let (tx, rx) = mpsc::channel(16);
        {
            let mut watchers = self.watchers.write().await;
            watchers
                .entry(service_name.to_string())
                .or_default()
                .push(tx.clone());
        }

        let initial_snapshot = self.discover(service_name).await?;
        let _ = tx.send(initial_snapshot).await;

        let mut watch_tasks = self.watch_tasks.write().await;
        if !watch_tasks.contains_key(service_name) {
            let service = service_name.to_string();
            let discovery = self.clone();
            let handle = tokio::spawn(async move {
                discovery.watch_loop(service).await;
            });
            watch_tasks.insert(service_name.to_string(), handle);
        }

        Ok(WatchStream { rx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_construction_uses_prefix() {
        assert_eq!(
            EtcdDiscovery::service_prefix_from("nova/services", "users"),
            "/nova/services/users/"
        );
        assert_eq!(
            EtcdDiscovery::instance_key_from("nova/services", "users", "users-1"),
            "/nova/services/users/users-1"
        );
    }

    #[test]
    fn stored_record_roundtrips() {
        let mut metadata = HashMap::new();
        metadata.insert("zone".to_string(), "a".to_string());

        let record = StoredServiceInstance {
            id: "users-1".to_string(),
            name: "users".to_string(),
            address: "127.0.0.1:9000".to_string(),
            metadata,
            status: InstanceStatus::Healthy,
            last_heartbeat_unix_ms: Some(EtcdDiscovery::now_millis()),
        };

        let payload = serde_json::to_vec(&record).expect("serialize record");
        let discovered = EtcdDiscovery::parse_service_instance(&payload).expect("parse record");

        assert_eq!(discovered.id, "users-1");
        assert_eq!(discovered.name, "users");
        assert_eq!(discovered.address, "127.0.0.1:9000");
        assert_eq!(
            discovered.metadata.get("zone").map(String::as_str),
            Some("a")
        );
        assert_eq!(discovered.status, InstanceStatus::Healthy);
        assert!(discovered.last_heartbeat.is_some());
    }

    #[tokio::test]
    async fn invalid_endpoint_returns_connection_failed() {
        let err = match EtcdDiscovery::new(vec!["http://[".to_string()], "nova/services", 15).await
        {
            Ok(_) => panic!("should fail to connect"),
            Err(err) => err,
        };

        assert!(matches!(err, DiscoveryError::ConnectionFailed(_)));
    }
}
