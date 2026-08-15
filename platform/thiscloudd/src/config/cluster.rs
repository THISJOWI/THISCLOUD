use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ClusterConfig {
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<NodeConfig>,
    #[serde(default)]
    pub etcd: EtcdConfig,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            name: "thiscloud".to_string(),
            nodes: Vec::new(),
            etcd: EtcdConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeConfig {
    pub ip: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EtcdConfig {
    #[serde(default = "default_embedded")]
    pub embedded: bool,
    /// External etcd endpoints (e.g. multi-master RAFT). When `embedded` is
    /// false the daemon connects to the first reachable endpoint.
    #[serde(default)]
    pub endpoints: Vec<String>,
    #[serde(default = "default_etcd_port")]
    pub port: u16,
    #[serde(default = "default_peer_port")]
    pub peer_port: u16,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_quota_backend")]
    pub quota_backend: String,
}

fn default_embedded() -> bool {
    true
}

fn default_etcd_port() -> u16 {
    2379
}

fn default_peer_port() -> u16 {
    2380
}

fn default_data_dir() -> String {
    "/var/lib/thiscloud/etcd".to_string()
}

fn default_quota_backend() -> String {
    "8GB".to_string()
}

impl Default for EtcdConfig {
    fn default() -> Self {
        Self {
            embedded: default_embedded(),
            endpoints: Vec::new(),
            port: default_etcd_port(),
            peer_port: default_peer_port(),
            data_dir: default_data_dir(),
            quota_backend: default_quota_backend(),
        }
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".to_string(),
            role: "master".to_string(),
        }
    }
}

impl ClusterConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
