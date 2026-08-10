pub mod cluster;
pub mod compute;
pub mod marketplace;
pub mod network;
pub mod storage;

pub use cluster::{ClusterConfig, EtcdConfig, NodeConfig};
pub use compute::ComputeConfig;
pub use marketplace::MarketplaceConfig;
pub use network::NetworkConfig;
pub use storage::{PoolConfig, StorageConfig};

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ThisCloudConfig {
    #[serde(default)]
    pub cluster: ClusterConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub compute: ComputeConfig,
    #[serde(default)]
    pub marketplace: MarketplaceConfig,
}

impl ThisCloudConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
