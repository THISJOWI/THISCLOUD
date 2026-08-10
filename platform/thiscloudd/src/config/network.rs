use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_management_vlan")]
    pub management_vlan: u16,
    #[serde(default = "default_overlay_type")]
    pub overlay_type: String,
    #[serde(default = "default_backend")]
    pub backend: String,
}

fn default_management_vlan() -> u16 {
    100
}

fn default_overlay_type() -> String {
    "geneve".to_string()
}

fn default_backend() -> String {
    "mock".to_string()
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            management_vlan: default_management_vlan(),
            overlay_type: default_overlay_type(),
            backend: default_backend(),
        }
    }
}

impl NetworkConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
