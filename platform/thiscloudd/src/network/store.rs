use crate::core::EtcdClient;
use crate::network::{DhcpServer, FloatingIp, LogicalNetwork, VirtualRouter};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait NetworkStore: Send + Sync {
    async fn put(&self, tenant_id: &str, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<LogicalNetwork>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<LogicalNetwork>>;
    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryNetworkStore {
    networks: Arc<Mutex<HashMap<String, LogicalNetwork>>>,
}

impl MemoryNetworkStore {
    fn composite_key(tenant_id: &str, id: &str) -> String {
        format!("{}:{}", tenant_id, id)
    }
}

#[async_trait::async_trait]
impl NetworkStore for MemoryNetworkStore {
    async fn put(&self, tenant_id: &str, net: &LogicalNetwork) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &net.id);
        self.networks.lock().unwrap().insert(key, net.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<LogicalNetwork>> {
        let key = Self::composite_key(tenant_id, id);
        Ok(self.networks.lock().unwrap().get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<LogicalNetwork>> {
        let store = self.networks.lock().unwrap();
        if tenant_id.is_empty() {
            Ok(store.values().cloned().collect())
        } else {
            let prefix = format!("{}:", tenant_id);
            Ok(store
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_, v)| v.clone())
                .collect())
        }
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, id);
        self.networks.lock().unwrap().remove(&key);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdNetworkStore {
    client: EtcdClient,
}

impl EtcdNetworkStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/networks/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/networks/")
        }
    }
}

#[async_trait::async_trait]
impl NetworkStore for EtcdNetworkStore {
    async fn put(&self, tenant_id: &str, net: &LogicalNetwork) -> anyhow::Result<()> {
        let json = serde_json::to_string(net)?;
        self.client.put(&Self::key(tenant_id, &net.id), &json).await
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<LogicalNetwork>> {
        match self.client.get(&Self::key(tenant_id, id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<LogicalNetwork>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut nets = Vec::new();
        for (_, json) in entries {
            if let Ok(net) = serde_json::from_str::<LogicalNetwork>(&json) {
                if tenant_id.is_empty() || net.tenant_id == tenant_id {
                    nets.push(net);
                }
            }
        }
        Ok(nets)
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, id)).await
    }
}

// ---------------------------------------------------------------------------
// Virtual routers
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait RouterStore: Send + Sync {
    async fn put(&self, tenant_id: &str, router: &VirtualRouter) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<VirtualRouter>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<VirtualRouter>>;
    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryRouterStore {
    routers: Arc<Mutex<HashMap<String, VirtualRouter>>>,
}

impl MemoryRouterStore {
    fn composite_key(tenant_id: &str, id: &str) -> String {
        format!("{}:{}", tenant_id, id)
    }
}

#[async_trait::async_trait]
impl RouterStore for MemoryRouterStore {
    async fn put(&self, tenant_id: &str, router: &VirtualRouter) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &router.id);
        self.routers.lock().unwrap().insert(key, router.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<VirtualRouter>> {
        let key = Self::composite_key(tenant_id, id);
        Ok(self.routers.lock().unwrap().get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<VirtualRouter>> {
        let store = self.routers.lock().unwrap();
        if tenant_id.is_empty() {
            Ok(store.values().cloned().collect())
        } else {
            let prefix = format!("{}:", tenant_id);
            Ok(store
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_, v)| v.clone())
                .collect())
        }
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, id);
        self.routers.lock().unwrap().remove(&key);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdRouterStore {
    client: EtcdClient,
}

impl EtcdRouterStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/network/routers/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/network/routers/")
        }
    }
}

#[async_trait::async_trait]
impl RouterStore for EtcdRouterStore {
    async fn put(&self, tenant_id: &str, router: &VirtualRouter) -> anyhow::Result<()> {
        let json = serde_json::to_string(router)?;
        self.client
            .put(&Self::key(tenant_id, &router.id), &json)
            .await
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<VirtualRouter>> {
        match self.client.get(&Self::key(tenant_id, id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<VirtualRouter>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut routers = Vec::new();
        for (_, json) in entries {
            if let Ok(router) = serde_json::from_str::<VirtualRouter>(&json) {
                if tenant_id.is_empty() || router.tenant_id == tenant_id {
                    routers.push(router);
                }
            }
        }
        Ok(routers)
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, id)).await
    }
}

// ---------------------------------------------------------------------------
// DHCP servers
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait DhcpStore: Send + Sync {
    async fn put(&self, tenant_id: &str, dhcp: &DhcpServer) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<DhcpServer>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<DhcpServer>>;
    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryDhcpStore {
    dhcp: Arc<Mutex<HashMap<String, DhcpServer>>>,
}

impl MemoryDhcpStore {
    fn composite_key(tenant_id: &str, id: &str) -> String {
        format!("{}:{}", tenant_id, id)
    }
}

#[async_trait::async_trait]
impl DhcpStore for MemoryDhcpStore {
    async fn put(&self, tenant_id: &str, dhcp: &DhcpServer) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &dhcp.id);
        self.dhcp.lock().unwrap().insert(key, dhcp.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<DhcpServer>> {
        let key = Self::composite_key(tenant_id, id);
        Ok(self.dhcp.lock().unwrap().get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<DhcpServer>> {
        let store = self.dhcp.lock().unwrap();
        if tenant_id.is_empty() {
            Ok(store.values().cloned().collect())
        } else {
            let prefix = format!("{}:", tenant_id);
            Ok(store
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_, v)| v.clone())
                .collect())
        }
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, id);
        self.dhcp.lock().unwrap().remove(&key);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdDhcpStore {
    client: EtcdClient,
}

impl EtcdDhcpStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/network/dhcp/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/network/dhcp/")
        }
    }
}

#[async_trait::async_trait]
impl DhcpStore for EtcdDhcpStore {
    async fn put(&self, tenant_id: &str, dhcp: &DhcpServer) -> anyhow::Result<()> {
        let json = serde_json::to_string(dhcp)?;
        self.client.put(&Self::key(tenant_id, &dhcp.id), &json).await
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<DhcpServer>> {
        match self.client.get(&Self::key(tenant_id, id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<DhcpServer>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut servers = Vec::new();
        for (_, json) in entries {
            if let Ok(dhcp) = serde_json::from_str::<DhcpServer>(&json) {
                if tenant_id.is_empty() || dhcp.tenant_id == tenant_id {
                    servers.push(dhcp);
                }
            }
        }
        Ok(servers)
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, id)).await
    }
}

// ---------------------------------------------------------------------------
// Floating IPs
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait FloatingIpStore: Send + Sync {
    async fn put(&self, tenant_id: &str, fip: &FloatingIp) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<FloatingIp>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<FloatingIp>>;
    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryFloatingIpStore {
    floating_ips: Arc<Mutex<HashMap<String, FloatingIp>>>,
}

impl MemoryFloatingIpStore {
    fn composite_key(tenant_id: &str, id: &str) -> String {
        format!("{}:{}", tenant_id, id)
    }
}

#[async_trait::async_trait]
impl FloatingIpStore for MemoryFloatingIpStore {
    async fn put(&self, tenant_id: &str, fip: &FloatingIp) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &fip.id);
        self.floating_ips.lock().unwrap().insert(key, fip.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<FloatingIp>> {
        let key = Self::composite_key(tenant_id, id);
        Ok(self.floating_ips.lock().unwrap().get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<FloatingIp>> {
        let store = self.floating_ips.lock().unwrap();
        if tenant_id.is_empty() {
            Ok(store.values().cloned().collect())
        } else {
            let prefix = format!("{}:", tenant_id);
            Ok(store
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_, v)| v.clone())
                .collect())
        }
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, id);
        self.floating_ips.lock().unwrap().remove(&key);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdFloatingIpStore {
    client: EtcdClient,
}

impl EtcdFloatingIpStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/network/floating-ips/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/network/floating-ips/")
        }
    }
}

#[async_trait::async_trait]
impl FloatingIpStore for EtcdFloatingIpStore {
    async fn put(&self, tenant_id: &str, fip: &FloatingIp) -> anyhow::Result<()> {
        let json = serde_json::to_string(fip)?;
        self.client
            .put(&Self::key(tenant_id, &fip.id), &json)
            .await
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<FloatingIp>> {
        match self.client.get(&Self::key(tenant_id, id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<FloatingIp>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut fips = Vec::new();
        for (_, json) in entries {
            if let Ok(fip) = serde_json::from_str::<FloatingIp>(&json) {
                if tenant_id.is_empty() || fip.tenant_id == tenant_id {
                    fips.push(fip);
                }
            }
        }
        Ok(fips)
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, id)).await
    }
}
