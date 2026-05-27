use async_trait::async_trait;
use nova_core::discovery::{
    Discovery, DiscoveryError, InstanceStatus, ServiceInstance, WatchStream,
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, warn};

type WatchersMap = HashMap<String, Vec<mpsc::Sender<Vec<ServiceInstance>>>>;

#[derive(Clone)]
pub struct ConsulDiscovery {
    client: reqwest::Client,
    base_url: String,
    datacenter: Option<String>,
    token: Option<String>,
    // Internal: tracks watch channels per service.
    watchers: Arc<RwLock<WatchersMap>>,
    watch_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl ConsulDiscovery {
    pub fn new(
        base_url: impl Into<String>,
        datacenter: Option<String>,
        token: Option<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            datacenter,
            token,
            watchers: Arc::new(RwLock::new(HashMap::new())),
            watch_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut builder = self.client.request(method, self.url(path));
        if let Some(dc) = &self.datacenter {
            builder = builder.query(&[("dc", dc)]);
        }
        if let Some(token) = &self.token {
            builder = builder.header("X-Consul-Token", token);
        }
        builder
    }

    async fn send_request(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, DiscoveryError> {
        let response = builder
            .send()
            .await
            .map_err(|e| DiscoveryError::Backend(e.to_string()))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(DiscoveryError::NotFound(
                "consul resource not found".to_string(),
            ));
        }

        if !response.status().is_success() {
            return Err(DiscoveryError::Backend(format!(
                "consul request failed with status {}",
                response.status()
            )));
        }

        Ok(response)
    }

    fn split_address(address: &str) -> Result<(String, u16), DiscoveryError> {
        let (host, port) = address
            .rsplit_once(':')
            .ok_or_else(|| DiscoveryError::Backend(format!("invalid address: {address}")))?;
        let port = port
            .parse::<u16>()
            .map_err(|e| DiscoveryError::Backend(format!("invalid address port: {e}")))?;
        Ok((host.to_string(), port))
    }

    fn metadata_from_value(value: Option<&JsonValue>) -> HashMap<String, String> {
        match value.and_then(JsonValue::as_object) {
            Some(map) => map
                .iter()
                .map(|(key, value)| {
                    let rendered = value
                        .as_str()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| value.to_string());
                    (key.clone(), rendered)
                })
                .collect(),
            None => HashMap::new(),
        }
    }

    fn service_instance_from_consul(
        service: &ConsulServiceEntry,
        status: InstanceStatus,
    ) -> Result<ServiceInstance, DiscoveryError> {
        let address = if service.service.address.is_empty() {
            service.node.address.clone()
        } else {
            service.service.address.clone()
        };
        let address = format!("{}:{}", address, service.service.port);
        let metadata = Self::metadata_from_value(service.service.meta.as_ref());

        Ok(ServiceInstance {
            id: service.service.id.clone(),
            name: service.service.service.clone(),
            address,
            metadata,
            status,
            last_heartbeat: None,
        })
    }

    async fn notify_watchers(&self, service_name: &str, instances: Vec<ServiceInstance>) {
        let watchers = {
            let watchers = self.watchers.read().await;
            watchers.get(service_name).cloned().unwrap_or_default()
        };

        if watchers.is_empty() {
            return;
        }

        let mut alive = Vec::with_capacity(watchers.len());
        for watcher in watchers {
            if watcher.send(instances.clone()).await.is_ok() {
                alive.push(watcher);
            }
        }

        let mut watchers_map = self.watchers.write().await;
        if let Some(entry) = watchers_map.get_mut(service_name) {
            *entry = alive;
        }
    }

    async fn current_instances(
        &self,
        service_name: &str,
    ) -> Result<Vec<ServiceInstance>, DiscoveryError> {
        self.discover(service_name).await
    }

    async fn watch_loop(self, service_name: String) {
        let mut last_index: u64 = 0;

        loop {
            let has_watchers = {
                let watchers = self.watchers.read().await;
                watchers
                    .get(&service_name)
                    .map(|items| !items.is_empty())
                    .unwrap_or(false)
            };

            if !has_watchers {
                break;
            }

            match self.long_poll(&service_name, last_index).await {
                Ok((instances, index)) => {
                    if index > last_index {
                        last_index = index;
                        self.notify_watchers(&service_name, instances).await;
                    }
                }
                Err(err) => {
                    warn!(service = %service_name, error = %err, "Consul watch loop error");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }

        self.watch_tasks.write().await.remove(&service_name);
    }

    async fn long_poll(
        &self,
        service_name: &str,
        last_index: u64,
    ) -> Result<(Vec<ServiceInstance>, u64), DiscoveryError> {
        let mut request = self
            .request(
                reqwest::Method::GET,
                &format!("/v1/health/service/{service_name}"),
            )
            .query(&[("passing", "true"), ("wait", "30s")]);
        if last_index > 0 {
            request = request.query(&[("index", &last_index.to_string())]);
        }

        let response = self.send_request(request).await?;
        let index = response
            .headers()
            .get("X-Consul-Index")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(last_index);
        let entries = response
            .json::<Vec<ConsulServiceEntry>>()
            .await
            .map_err(|e| DiscoveryError::Backend(e.to_string()))?;
        let instances = entries
            .iter()
            .map(|entry| Self::service_instance_from_consul(entry, InstanceStatus::Healthy))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((instances, index))
    }
}

#[derive(Debug, Deserialize)]
struct ConsulNodeEntry {
    #[serde(rename = "Address")]
    address: String,
}

#[derive(Debug, Deserialize)]
struct ConsulServiceInfo {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Service")]
    service: String,
    #[serde(rename = "Address", default)]
    address: String,
    #[serde(rename = "Port")]
    port: u16,
    #[serde(rename = "Meta", default)]
    meta: Option<JsonValue>,
}

#[derive(Debug, Deserialize)]
struct ConsulServiceEntry {
    #[serde(rename = "Node")]
    node: ConsulNodeEntry,
    #[serde(rename = "Service")]
    service: ConsulServiceInfo,
}

#[async_trait]
impl Discovery for ConsulDiscovery {
    async fn register(&self, instance: ServiceInstance) -> Result<(), DiscoveryError> {
        let (address, port) = Self::split_address(&instance.address)?;
        let check_id = format!("service:{}", instance.id);
        let payload = serde_json::json!({
            "ID": instance.id,
            "Name": instance.name,
            "Address": address,
            "Port": port,
            "Meta": instance.metadata,
            "Check": {
                "CheckID": check_id,
                "TTL": "30s",
                "DeregisterCriticalServiceAfter": "90s"
            }
        });

        self.send_request(
            self.request(reqwest::Method::PUT, "/v1/agent/service/register")
                .json(&payload),
        )
        .await?;

        self.notify_watchers(
            &instance.name,
            self.current_instances(&instance.name).await?,
        )
        .await;
        Ok(())
    }

    async fn discover(&self, service_name: &str) -> Result<Vec<ServiceInstance>, DiscoveryError> {
        let response = self
            .send_request(
                self.request(
                    reqwest::Method::GET,
                    &format!("/v1/health/service/{service_name}"),
                )
                .query(&[("passing", "true")]),
            )
            .await?;

        let entries = response
            .json::<Vec<ConsulServiceEntry>>()
            .await
            .map_err(|e| DiscoveryError::Backend(e.to_string()))?;

        entries
            .iter()
            .map(|entry| Self::service_instance_from_consul(entry, InstanceStatus::Healthy))
            .collect()
    }

    async fn heartbeat(&self, service_name: &str, instance_id: &str) -> Result<(), DiscoveryError> {
        let check_id = format!("service:{instance_id}");
        debug!(service = %service_name, instance = %instance_id, check_id = %check_id, "sending consul heartbeat");
        self.send_request(self.request(
            reqwest::Method::PUT,
            &format!("/v1/agent/check/pass/{check_id}"),
        ))
        .await?;
        Ok(())
    }

    async fn deregister(
        &self,
        service_name: &str,
        instance_id: &str,
    ) -> Result<(), DiscoveryError> {
        self.send_request(self.request(
            reqwest::Method::PUT,
            &format!("/v1/agent/service/deregister/{instance_id}"),
        ))
        .await?;

        self.notify_watchers(service_name, self.current_instances(service_name).await?)
            .await;
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

        let initial = self.discover(service_name).await?;
        let _ = tx.send(initial).await;

        let mut tasks = self.watch_tasks.write().await;
        if !tasks.contains_key(service_name) {
            let service = service_name.to_string();
            let discovery = self.clone();
            let handle = tokio::spawn(async move {
                discovery.watch_loop(service).await;
            });
            tasks.insert(service_name.to_string(), handle);
        }

        Ok(WatchStream { rx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_core::discovery::DiscoveryError;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn discovery() -> ConsulDiscovery {
        ConsulDiscovery::new("http://127.0.0.1:1", None, None)
    }

    #[tokio::test]
    async fn invalid_url_returns_backend_error() {
        let discovery = ConsulDiscovery::new("http://[", None, None);
        let err = discovery.discover("users").await.expect_err("should fail");
        assert!(matches!(err, DiscoveryError::Backend(_)));
    }

    #[tokio::test]
    async fn discover_404_maps_to_not_found() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("local addr");

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let response =
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = socket.write_all(response).await;
                let _ = socket.shutdown().await;
            }
        });

        let discovery = ConsulDiscovery::new(format!("http://{addr}"), None, None);
        let err = discovery.discover("users").await.expect_err("should fail");
        assert!(matches!(err, DiscoveryError::NotFound(_)));
    }

    #[tokio::test]
    async fn register_invalid_address_returns_backend_error() {
        let discovery = discovery();
        let instance = ServiceInstance {
            id: "users-1".to_string(),
            name: "users".to_string(),
            address: "invalid-address".to_string(),
            metadata: HashMap::new(),
            status: InstanceStatus::Healthy,
            last_heartbeat: None,
        };

        let err = discovery.register(instance).await.expect_err("should fail");
        assert!(matches!(err, DiscoveryError::Backend(_)));
    }
}
