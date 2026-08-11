use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NetworkStatus {
    #[default]
    Created,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalNetwork {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub cidr: String,
    pub gateway: String,
    #[serde(default)]
    pub vlan: Option<u16>,
    #[serde(default)]
    pub dns: Vec<String>,
    #[serde(default)]
    pub status: NetworkStatus,
    #[serde(default)]
    pub tenant_id: String,
}

impl LogicalNetwork {
    pub fn new(id: String, name: String, cidr: String, gateway: String) -> Self {
        Self {
            id,
            name,
            cidr,
            gateway,
            vlan: None,
            dns: Vec::new(),
            status: NetworkStatus::Created,
            tenant_id: String::new(),
        }
    }
}
