pub mod auth;
pub mod cluster;
pub mod compute;
pub mod ha;
pub mod image;
pub mod marketplace;
pub mod metrics;
pub mod network;
pub mod node;
pub mod s3;
pub mod storage;

pub use auth::AuthConfig;
pub use cluster::{ClusterConfig, EtcdConfig, NodeConfig};
pub use compute::ComputeConfig;
pub use ha::HaConfig;
pub use image::ImageConfig;
pub use marketplace::MarketplaceConfig;
pub use metrics::MetricsConfig;
pub use network::NetworkConfig;
pub use node::NodeIdentityConfig;
pub use s3::S3Config;
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
    #[serde(default)]
    pub image: ImageConfig,
    #[serde(default)]
    pub s3: S3Config,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub ha: HaConfig,
    #[serde(default)]
    pub node: NodeIdentityConfig,
}

impl ThisCloudConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
