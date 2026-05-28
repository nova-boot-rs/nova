use nova_boot::{Deserialize, ReloadableConfig, Serialize};

// The app state carries the live config handle so handlers and startup code
// can share the same reloaded settings without manual file polling.
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

// `AppState` is intentionally small in the demo so the focus stays on the
// framework features rather than bespoke state management.
#[derive(Clone)]
pub struct AppState {
    pub runtime_config: ReloadableConfig<RuntimeConfig>,
}
