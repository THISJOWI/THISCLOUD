use serde::Deserialize;

/// T5.1 metrics configuration: whether the Prometheus scrape endpoint is
/// enabled and where the daemon's metrics listener binds.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MetricsConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_listen")]
    pub listen: String,
}

fn default_enabled() -> bool {
    true
}

fn default_listen() -> String {
    "127.0.0.1:9100".to_string()
}