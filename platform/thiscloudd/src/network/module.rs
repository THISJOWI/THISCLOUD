use crate::core::{Event, EventBus};
use crate::network::{LogicalNetwork, NetworkBackend, NetworkStore};

pub struct NetworkModule {
    backend: Box<dyn NetworkBackend>,
    store: Box<dyn NetworkStore>,
}

impl NetworkModule {
    pub fn new(backend: Box<dyn NetworkBackend>, store: Box<dyn NetworkStore>) -> Self {
        Self { backend, store }
    }

    pub async fn create_network(&mut self, net: &mut LogicalNetwork) -> anyhow::Result<()> {
        for existing in self.store.list().await? {
            if existing.name == net.name {
                anyhow::bail!("network '{}' already exists", net.name);
            }
        }
        if net.id.is_empty() {
            net.id = uuid::Uuid::new_v4().to_string();
        }
        self.store.put(net).await?;
        self.backend.create(net).await?;
        tracing::info!("Network created: {} ({})", net.name, net.id);
        Ok(())
    }

    pub async fn get_network(&self, id: &str) -> anyhow::Result<LogicalNetwork> {
        self.store
            .get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("network {} not found", id))
    }

    pub async fn list_networks(&self) -> anyhow::Result<Vec<LogicalNetwork>> {
        self.store.list().await
    }

    pub async fn delete_network(&mut self, id: &str) -> anyhow::Result<()> {
        let net = self.get_network(id).await?;
        self.backend.delete(&net).await?;
        self.store.delete(id).await?;
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
    pub fn publish_event(&self, _event_bus: &EventBus, _event: Event) {
        // Reserved: emits events once HTTP layer is wired.
    }
}
