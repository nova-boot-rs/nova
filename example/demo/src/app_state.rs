use nova_core::{Deserialize, ReloadableConfig, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub app_label: String,
    pub maintenance_mode: bool,
    pub sample_rate: f64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            app_label: "Nova Demo".to_string(),
            maintenance_mode: false,
            sample_rate: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub runtime_config: ReloadableConfig<RuntimeConfig>,
}
