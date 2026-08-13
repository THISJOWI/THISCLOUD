use crate::core::{Event, EventBus};
use crate::network::{LogicalNetwork, NetworkBackend, NetworkStore};
use crate::quota::model::ResourceDelta;
use crate::quota::QuotaModule;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct NetworkModule {
    backend: Box<dyn NetworkBackend>,
    store: Box<dyn NetworkStore>,
    quota: Option<Arc<Mutex<QuotaModule>>>,
}

impl NetworkModule {
    pub fn new(backend: Box<dyn NetworkBackend>, store: Box<dyn NetworkStore>) -> Self {
        Self {
            backend,
            store,
            quota: None,
        }
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
