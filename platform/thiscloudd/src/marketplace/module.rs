use crate::core::Module;
use crate::marketplace::{MarketplaceApp, MarketplaceBackend, MarketplaceStatus, MarketplaceStore};
use async_trait::async_trait;

pub struct MarketplaceModule {
    backend: Box<dyn MarketplaceBackend>,
    store: Box<dyn MarketplaceStore>,
}

impl MarketplaceModule {
    pub fn new(backend: Box<dyn MarketplaceBackend>, store: Box<dyn MarketplaceStore>) -> Self {
        Self { backend, store }
    }

    pub async fn install(&mut self, tenant_id: &str, app: &mut MarketplaceApp) -> anyhow::Result<MarketplaceApp> {
        if app.name.is_empty() {
            anyhow::bail!("app name is required");
        }
        if app.id.is_empty() {
            app.id = uuid::Uuid::new_v4().to_string();
        }
        app.tenant_id = tenant_id.to_string();
        self.backend.install(app).await?;
        app.status = MarketplaceStatus::Installed;
        self.store.put(tenant_id, app).await?;
        Ok(app.clone())
    }

    pub async fn uninstall(&mut self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let app = self
            .store
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("app {id} not found"))?;
        self.backend.uninstall(&app).await?;
        self.store.delete(tenant_id, id).await?;
        Ok(())
    }

    pub async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<MarketplaceApp> {
        self.store
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("app {id} not found"))
    }

    pub async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<MarketplaceApp>> {
        self.store.list(tenant_id).await
    }
}

#[async_trait]
impl Module for MarketplaceModule {
    fn name(&self) -> &str {
        "marketplace"
    }

    async fn start(&mut self, _event_bus: &crate::core::EventBus) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}
