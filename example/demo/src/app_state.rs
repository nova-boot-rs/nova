use nova_boot::{Deserialize, ReloadableConfig, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub app_label: String,
    pub maintenance_mode: bool,
    pub sample_rate: f64,
    pub replicas: Vec<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            app_label: "Nova Demo".to_string(),
            maintenance_mode: false,
            sample_rate: 1.0,
            replicas: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub runtime_config: ReloadableConfig<RuntimeConfig>,
}
