use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PoolType {
    #[default]
    Linstor,
    Drbd,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePool {
    pub name: String,
    pub pool_type: PoolType,
    #[serde(default)]
    pub devices: Vec<String>,
    #[serde(default = "default_replication")]
    pub replication: u32,
}

fn default_replication() -> u32 {
    2
}

impl StoragePool {
    pub fn new(name: String, pool_type: PoolType, devices: Vec<String>, replication: u32) -> Self {
        Self {
            name,
            pool_type,
            devices,
            replication,
        }
    }
}
