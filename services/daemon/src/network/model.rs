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

/// Virtual router: routes between tenant networks and optionally an external
/// network. `ha` requests high-availability (active/passive) placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirtualRouter {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub net_id: Option<String>,
    #[serde(default)]
    pub external_net_id: Option<String>,
    #[serde(default)]
    pub ha: bool,
    #[serde(default)]
    pub tenant_id: String,
    #[serde(default)]
    pub status: NetworkStatus,
}

impl VirtualRouter {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            net_id: None,
            external_net_id: None,
            ha: false,
            tenant_id: String::new(),
            status: NetworkStatus::Created,
        }
    }
}

/// DHCP server serving a tenant network. `pool_start`/`pool_end` bound the
/// dynamic range; `dns` lists the DNS servers handed out to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhcpServer {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub net_id: String,
    pub pool_start: String,
    pub pool_end: String,
    #[serde(default)]
    pub dns: Vec<String>,
    #[serde(default)]
    pub tenant_id: String,
    #[serde(default)]
    pub status: NetworkStatus,
}

impl DhcpServer {
    pub fn new(
        id: String,
        name: String,
        net_id: String,
        pool_start: String,
        pool_end: String,
    ) -> Self {
        Self {
            id,
            name,
            net_id,
            pool_start,
            pool_end,
            dns: Vec::new(),
            tenant_id: String::new(),
            status: NetworkStatus::Created,
        }
    }
}

/// Floating IP: a routable address mapped to a VM on a tenant network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloatingIp {
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// Empty when the caller wants the module to pick a free address from the
    /// network's CIDR.
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub vm_id: Option<String>,
    #[serde(default)]
    pub net_id: Option<String>,
    #[serde(default)]
    pub tenant_id: String,
}

impl FloatingIp {
    pub fn new(id: String, name: String, ip: String) -> Self {
        Self {
            id,
            name,
            ip,
            vm_id: None,
            net_id: None,
            tenant_id: String::new(),
        }
    }
}
