use crate::core::{EtcdClient, Event, EventBus};
use crate::network::{
    DhcpServer, DhcpStore, FloatingIp, FloatingIpStore, LogicalNetwork, NetworkBackend,
    NetworkStore, VirtualRouter, RouterStore,
};
use crate::quota::model::ResourceDelta;
use crate::quota::QuotaModule;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct NetworkModule {
    backend: Box<dyn NetworkBackend>,
    store: Box<dyn NetworkStore>,
    routers: Box<dyn RouterStore>,
    dhcp: Box<dyn DhcpStore>,
    floating_ips: Box<dyn FloatingIpStore>,
    quota: Option<Arc<Mutex<QuotaModule>>>,
}

impl NetworkModule {
    pub fn new(backend: Box<dyn NetworkBackend>, store: Box<dyn NetworkStore>) -> Self {
        Self {
            backend,
            store,
            routers: Box::new(crate::network::MemoryRouterStore::default()),
            dhcp: Box::new(crate::network::MemoryDhcpStore::default()),
            floating_ips: Box::new(crate::network::MemoryFloatingIpStore::default()),
            quota: None,
        }
    }

    /// Persist VPC resources (routers, DHCP, floating IPs) in etcd instead of
    /// the default in-memory stores.
    pub fn with_etcd_stores(mut self, client: EtcdClient) -> Self {
        self.routers = Box::new(crate::network::EtcdRouterStore::new(client.clone()));
        self.dhcp = Box::new(crate::network::EtcdDhcpStore::new(client.clone()));
        self.floating_ips = Box::new(crate::network::EtcdFloatingIpStore::new(client));
        self
    }

    /// Enable quota enforcement for this module (T0.5).
    pub fn with_quota(mut self, quota: Arc<Mutex<QuotaModule>>) -> Self {
        self.quota = Some(quota);
        self
    }

    /// Enforce tenant quota on the number of logical networks.
    async fn enforce_quota(&self, tenant_id: &str) -> anyhow::Result<()> {
        if let Some(quota) = &self.quota {
            let existing = self.store.list(tenant_id).await?;
            let delta = ResourceDelta {
                networks: existing.len() as u32 + 1,
                ..Default::default()
            };
            quota.lock().await.check(tenant_id, &delta).await?;
        }
        Ok(())
    }

    pub async fn create_network(&mut self, tenant_id: &str, net: &mut LogicalNetwork) -> anyhow::Result<()> {
        net.tenant_id = tenant_id.to_string();
        self.enforce_quota(tenant_id).await?;
        for existing in self.store.list(tenant_id).await? {
            if existing.name == net.name {
                anyhow::bail!("network '{}' already exists", net.name);
            }
        }
        if net.id.is_empty() {
            net.id = uuid::Uuid::new_v4().to_string();
        }
        self.store.put(tenant_id, net).await?;
        self.backend.create(net).await?;
        tracing::info!("Network created: {} ({})", net.name, net.id);
        Ok(())
    }

    pub async fn get_network(&self, tenant_id: &str, id: &str) -> anyhow::Result<LogicalNetwork> {
        self.store
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("network {} not found", id))
    }

    pub async fn list_networks(&self, tenant_id: &str) -> anyhow::Result<Vec<LogicalNetwork>> {
        self.store.list(tenant_id).await
    }

    pub async fn delete_network(&mut self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let net = self.get_network(tenant_id, id).await?;
        self.backend.delete(&net).await?;
        self.store.delete(tenant_id, id).await?;
        tracing::info!("Network deleted: {}", net.name);
        Ok(())
    }

    // --- Virtual routers ---

    pub async fn create_router(
        &mut self,
        tenant_id: &str,
        router: &mut VirtualRouter,
    ) -> anyhow::Result<()> {
        router.tenant_id = tenant_id.to_string();
        for existing in self.routers.list(tenant_id).await? {
            if existing.name == router.name {
                anyhow::bail!("router '{}' already exists", router.name);
            }
        }
        if router.id.is_empty() {
            router.id = uuid::Uuid::new_v4().to_string();
        }
        self.routers.put(tenant_id, router).await?;
        tracing::info!("Router created: {} ({})", router.name, router.id);
        Ok(())
    }

    pub async fn get_router(&self, tenant_id: &str, id: &str) -> anyhow::Result<VirtualRouter> {
        self.routers
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("router {} not found", id))
    }

    pub async fn list_routers(&self, tenant_id: &str) -> anyhow::Result<Vec<VirtualRouter>> {
        self.routers.list(tenant_id).await
    }

    pub async fn delete_router(&mut self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let router = self.get_router(tenant_id, id).await?;
        self.routers.delete(tenant_id, id).await?;
        tracing::info!("Router deleted: {}", router.name);
        Ok(())
    }

    // --- DHCP servers ---

    pub async fn create_dhcp(
        &mut self,
        tenant_id: &str,
        dhcp: &mut DhcpServer,
    ) -> anyhow::Result<()> {
        dhcp.tenant_id = tenant_id.to_string();
        for existing in self.dhcp.list(tenant_id).await? {
            if existing.name == dhcp.name {
                anyhow::bail!("dhcp server '{}' already exists", dhcp.name);
            }
        }
        if dhcp.id.is_empty() {
            dhcp.id = uuid::Uuid::new_v4().to_string();
        }
        self.dhcp.put(tenant_id, dhcp).await?;
        tracing::info!("DHCP server created: {} ({})", dhcp.name, dhcp.id);
        Ok(())
    }

    pub async fn get_dhcp(&self, tenant_id: &str, id: &str) -> anyhow::Result<DhcpServer> {
        self.dhcp
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("dhcp server {} not found", id))
    }

    pub async fn list_dhcp(&self, tenant_id: &str) -> anyhow::Result<Vec<DhcpServer>> {
        self.dhcp.list(tenant_id).await
    }

    pub async fn delete_dhcp(&mut self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let dhcp = self.get_dhcp(tenant_id, id).await?;
        self.dhcp.delete(tenant_id, id).await?;
        tracing::info!("DHCP server deleted: {}", dhcp.name);
        Ok(())
    }

    // --- Floating IPs ---

    /// Allocate a floating IP. When `explicit_ip` is `None` (or the model's
    /// `ip` is empty) a free address is picked from the referenced network's
    /// CIDR, skipping the gateway and already-allocated addresses.
    pub async fn allocate_floating_ip(
        &mut self,
        tenant_id: &str,
        fip: &mut FloatingIp,
        explicit_ip: Option<String>,
    ) -> anyhow::Result<()> {
        fip.tenant_id = tenant_id.to_string();
        let existing = self.floating_ips.list(tenant_id).await?;
        for f in &existing {
            if f.name == fip.name {
                anyhow::bail!("floating ip '{}' already exists", fip.name);
            }
        }
        if let Some(ip) = explicit_ip {
            if existing.iter().any(|f| f.ip == ip) {
                anyhow::bail!("floating ip {} already allocated", ip);
            }
            fip.ip = ip;
        } else {
            let net_id = fip.net_id.as_deref().ok_or_else(|| {
                anyhow::anyhow!("net_id required to allocate a floating ip")
            })?;
            let net = self
                .store
                .get(tenant_id, net_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("network {} not found", net_id))?;
            let used: Vec<String> = existing.iter().map(|f| f.ip.clone()).collect();
            fip.ip = free_ipv4(&net.cidr, &net.gateway, &used)?;
        }
        if fip.id.is_empty() {
            fip.id = uuid::Uuid::new_v4().to_string();
        }
        self.floating_ips.put(tenant_id, fip).await?;
        tracing::info!("Floating IP allocated: {} ({})", fip.ip, fip.id);
        Ok(())
    }

    pub async fn get_floating_ip(&self, tenant_id: &str, id: &str) -> anyhow::Result<FloatingIp> {
        self.floating_ips
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("floating ip {} not found", id))
    }

    pub async fn list_floating_ips(&self, tenant_id: &str) -> anyhow::Result<Vec<FloatingIp>> {
        self.floating_ips.list(tenant_id).await
    }

    pub async fn deallocate_floating_ip(
        &mut self,
        tenant_id: &str,
        id: &str,
    ) -> anyhow::Result<()> {
        let fip = self.get_floating_ip(tenant_id, id).await?;
        self.floating_ips.delete(tenant_id, id).await?;
        tracing::info!("Floating IP deallocated: {}", fip.ip);
        Ok(())
    }
}

/// Pick the first free IPv4 host address in `cidr`, skipping the gateway and
/// any already-used addresses. Only IPv4 is supported for floating IPs.
fn free_ipv4(cidr: &str, gateway: &str, used: &[String]) -> anyhow::Result<String> {
    let (ip_str, prefix_str) = cidr
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("invalid cidr: {cidr}"))?;
    let prefix: u8 = prefix_str
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid prefix in cidr: {cidr}"))?;
    let base: u32 = ip_str
        .parse::<std::net::Ipv4Addr>()
        .map_err(|_| anyhow::anyhow!("cidr must be IPv4: {cidr}"))?
        .into();
    if prefix > 30 {
        anyhow::bail!("cidr {cidr} has no usable host addresses");
    }
    let host_count = 1u64 << (32 - prefix);
    let scan = host_count.saturating_sub(1).min(65536);
    let used_set: std::collections::HashSet<&str> = used.iter().map(String::as_str).collect();
    for i in 1..scan {
        let ip = std::net::Ipv4Addr::from(base.wrapping_add(i as u32));
        let s = ip.to_string();
        if s == gateway || used_set.contains(s.as_str()) {
            continue;
        }
        return Ok(s);
    }
    anyhow::bail!("no free addresses in {cidr}")
}

#[async_trait::async_trait]
impl crate::core::Module for NetworkModule {
    fn name(&self) -> &str {
        "network"
    }

    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Network module started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Network module stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}

impl NetworkModule {
    pub fn publish_event(&self, _event_bus: &EventBus, _event: Event) {}
}
