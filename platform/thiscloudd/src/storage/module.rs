use crate::core::{Event, EventBus};
use crate::storage::{StorageBackend, StoragePool, StorageStore};

pub struct StorageModule {
    backend: Box<dyn StorageBackend>,
    store: Box<dyn StorageStore>,
}

impl StorageModule {
    pub fn new(backend: Box<dyn StorageBackend>, store: Box<dyn StorageStore>) -> Self {
        Self { backend, store }
    }

    pub async fn create_pool(&mut self, pool: StoragePool) -> anyhow::Result<()> {
        for existing in self.store.list().await? {
            if existing.name == pool.name {
                anyhow::bail!("storage pool '{}' already exists", pool.name);
            }
        }
        self.store.put(&pool).await?;
        self.backend.create(&pool).await?;
        tracing::info!("Storage pool created: {}", pool.name);
        Ok(())
    }

    pub async fn get_pool(&self, name: &str) -> anyhow::Result<StoragePool> {
        self.store
            .get(name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("storage pool {} not found", name))
    }

    pub async fn list_pools(&self) -> anyhow::Result<Vec<StoragePool>> {
        self.store.list().await
    }

    pub async fn delete_pool(&mut self, name: &str) -> anyhow::Result<()> {
        let pool = self.get_pool(name).await?;
        self.backend.delete(&pool).await?;
        self.store.delete(name).await?;
        tracing::info!("Storage pool deleted: {}", pool.name);
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::core::Module for StorageModule {
    fn name(&self) -> &str {
        "storage"
    }

    async fn start(&mut self, _event_bus: &EventBus) -> anyhow::Result<()> {
        tracing::info!("Storage module started");
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Storage module stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}

impl StorageModule {
    pub fn publish_event(&self, _event_bus: &EventBus, _event: Event) {
        // Reserved: emits events once HTTP layer is wired.
    }
}
