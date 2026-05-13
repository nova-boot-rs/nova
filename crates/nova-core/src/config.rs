use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::NovaError;
use crate::NovaResult;

pub trait NovaConfigSource {
    fn load(&self) -> NovaResult<Value>;
}

pub trait NovaSecretSource {
    fn resolve(&self, key: &str) -> NovaResult<Option<String>>;
}

#[derive(Debug, Clone)]
pub struct NovaConfig<T> {
    pub inner: T,
}

impl<T> NovaConfig<T> {
    pub fn into_inner(self) -> T {
        self.inner
    }
}

pub struct NovaConfigBuilder<T> {
    defaults: Option<T>,
    sources: Vec<Box<dyn NovaConfigSource>>,
    secret_sources: Vec<Box<dyn NovaSecretSource>>,
}

impl<T> Default for NovaConfigBuilder<T> {
    fn default() -> Self {
        Self {
            defaults: None,
            sources: Vec::new(),
            secret_sources: Vec::new(),
        }
    }
}

impl<T> NovaConfigBuilder<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn defaults(mut self, defaults: T) -> Self {
        self.defaults = Some(defaults);
        self
    }

    pub fn with_source<S>(mut self, source: S) -> Self
    where
        S: NovaConfigSource + 'static,
    {
        self.sources.push(Box::new(source));
        self
    }

    pub fn with_secret_source<S>(mut self, source: S) -> Self
    where
        S: NovaSecretSource + 'static,
    {
        self.secret_sources.push(Box::new(source));
        self
    }

    pub fn with_env_prefix(self, prefix: impl Into<String>) -> Self {
        self.with_source(EnvConfigSource::new(prefix))
    }

    pub fn with_json_file(self, path: impl Into<PathBuf>) -> Self {
        self.with_source(JsonFileConfigSource::new(path))
    }

    pub fn build(self) -> NovaResult<NovaConfig<T>> {
        let mut merged = if let Some(defaults) = self.defaults {
            serde_json::to_value(defaults)?
        } else {
            Value::Object(Map::new())
        };

        for source in self.sources {
            let value = source.load()?;
            merge_value(&mut merged, value);
        }

        let resolved = resolve_secrets(merged, &self.secret_sources)?;
        let inner = serde_json::from_value(resolved)?;

        Ok(NovaConfig { inner })
    }
}

pub struct EnvConfigSource {
    prefix: String,
}

impl EnvConfigSource {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl NovaConfigSource for EnvConfigSource {
    fn load(&self) -> NovaResult<Value> {
        let mut root = Value::Object(Map::new());
        let prefix = self.prefix.to_uppercase();

        for (key, raw_value) in env::vars() {
            let normalized_key = key.to_uppercase();
            if !normalized_key.starts_with(&prefix) {
                continue;
            }

            let trimmed = normalized_key
                .trim_start_matches(&prefix)
                .trim_start_matches('_');

            if trimmed.is_empty() {
                continue;
            }

            let path: Vec<String> = trimmed
                .split("__")
                .filter(|segment| !segment.is_empty())
                .map(|segment| segment.to_lowercase())
                .collect();

            if path.is_empty() {
                continue;
            }

            set_value_at_path(&mut root, &path, parse_env_value(&raw_value));
        }

        Ok(root)
    }
}

pub struct JsonFileConfigSource {
    path: PathBuf,
}

impl JsonFileConfigSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl NovaConfigSource for JsonFileConfigSource {
    fn load(&self) -> NovaResult<Value> {
        let contents = fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&contents)?)
    }
}

fn merge_value(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(existing) => merge_value(existing, value),
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
        }
        (base_slot, overlay_value) => {
            *base_slot = overlay_value;
        }
    }
}

fn set_value_at_path(root: &mut Value, path: &[String], value: Value) {
    if path.is_empty() {
        *root = value;
        return;
    }

    let Some((head, tail)) = path.split_first() else {
        return;
    };

    if !root.is_object() {
        *root = Value::Object(Map::new());
    }

    let object = root.as_object_mut().expect("root must be object");

    if tail.is_empty() {
        object.insert(head.clone(), value);
        return;
    }

    let next = object
        .entry(head.clone())
        .or_insert_with(|| Value::Object(Map::new()));

    set_value_at_path(next, tail, value);
}

fn parse_env_value(raw_value: &str) -> Value {
    serde_json::from_str(raw_value).unwrap_or_else(|_| Value::String(raw_value.to_string()))
}

fn resolve_secrets(
    value: Value,
    secret_sources: &[Box<dyn NovaSecretSource>],
) -> NovaResult<Value> {
    match value {
        Value::String(text) => {
            if let Some(secret_key) = text.strip_prefix("secret://") {
                for source in secret_sources {
                    if let Some(secret_value) = source.resolve(secret_key)? {
                        return Ok(Value::String(secret_value));
                    }
                }

                Err(NovaError::NotFound(format!(
                    "secret '{secret_key}' was not resolved"
                )))
            } else {
                Ok(Value::String(text))
            }
        }
        Value::Array(items) => {
            let mut resolved = Vec::with_capacity(items.len());
            for item in items {
                resolved.push(resolve_secrets(item, secret_sources)?);
            }
            Ok(Value::Array(resolved))
        }
        Value::Object(map) => {
            let mut resolved = Map::new();
            for (key, item) in map {
                resolved.insert(key, resolve_secrets(item, secret_sources)?);
            }
            Ok(Value::Object(resolved))
        }
        other => Ok(other),
    }
}

#[derive(Debug, Clone)]
pub struct MapSecretSource {
    entries: std::collections::HashMap<String, String>,
}

impl MapSecretSource {
    pub fn new(entries: std::collections::HashMap<String, String>) -> Self {
        Self { entries }
    }
}

impl NovaSecretSource for MapSecretSource {
    fn resolve(&self, key: &str) -> NovaResult<Option<String>> {
        Ok(self.entries.get(key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fs;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestConfig {
        host: String,
        port: u16,
        nested: NestedConfig,
        secret: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct NestedConfig {
        enabled: bool,
    }

    #[test]
    fn layers_env_over_file_over_defaults() {
        let mut path = std::env::temp_dir();
        path.push(format!("nova-config-{}.json", std::process::id()));
        fs::write(
            &path,
            r#"{
                "host": "file-host",
                "nested": {"enabled": false},
                "secret": "secret://api_key"
            }"#,
        )
        .expect("write temp config");

        let mut secrets = HashMap::new();
        secrets.insert("api_key".to_string(), "resolved-secret".to_string());

        let config = NovaConfigBuilder::new()
            .defaults(TestConfig {
                host: "default-host".into(),
                port: 8080,
                nested: NestedConfig { enabled: true },
                secret: "default-secret".into(),
            })
            .with_json_file(&path)
            .with_secret_source(MapSecretSource::new(secrets))
            .build()
            .expect("build config");

        assert_eq!(config.inner.host, "file-host");
        assert_eq!(config.inner.port, 8080);
        assert!(!config.inner.nested.enabled);
        assert_eq!(config.inner.secret, "resolved-secret");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn env_source_supports_nested_paths() {
        let source = EnvConfigSource::new("NOVA_TEST");

        let mut root = source.load().expect("load env source");
        set_value_at_path(
            &mut root,
            &["nested".into(), "enabled".into()],
            Value::Bool(true),
        );

        assert_eq!(root["nested"]["enabled"], Value::Bool(true));
    }
}

// Resilience configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ResilienceBackend {
    Local,
    Redis { url: String, prefix: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub backend: ResilienceBackend,
    pub threshold: u32,
    /// TTL in seconds for open state when using distributed backend
    pub open_ttl_seconds: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            backend: ResilienceBackend::Local,
            threshold: 5,
            open_ttl_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterConfig {
    pub backend: ResilienceBackend,
    pub capacity: i64,
    pub window_seconds: usize,
    pub prefix: Option<String>,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            backend: ResilienceBackend::Local,
            capacity: 100,
            window_seconds: 60,
            prefix: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResilienceConfig {
    pub circuit_breaker: Option<CircuitBreakerConfig>,
    pub rate_limiter: Option<RateLimiterConfig>,
}
