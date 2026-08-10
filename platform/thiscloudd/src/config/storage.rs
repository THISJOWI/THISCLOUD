use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StorageConfig {
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default)]
    pub pools: Vec<PoolConfig>,
}

fn default_backend() -> String {
    "mock".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoolConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub pool_type: String,
    #[serde(default)]
    pub devices: Vec<String>,
    #[serde(default = "default_replication")]
    pub replication: u32,
}

fn default_replication() -> u32 {
    2
}

impl StorageConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
