use crate::core::EtcdClient;
use crate::marketplace::MarketplaceApp;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait MarketplaceStore: Send + Sync {
    async fn put(&self, app: &MarketplaceApp) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<MarketplaceApp>>;
    async fn list(&self) -> anyhow::Result<Vec<MarketplaceApp>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
}

/// In-memory store used by tests and macOS dev.
#[derive(Debug, Default)]
pub struct MemoryMarketplaceStore {
    apps: Arc<Mutex<HashMap<String, MarketplaceApp>>>,
}

#[async_trait]
impl MarketplaceStore for MemoryMarketplaceStore {
    async fn put(&self, app: &MarketplaceApp) -> anyhow::Result<()> {
        let mut apps = self
            .apps
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        apps.insert(app.id.clone(), app.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<MarketplaceApp>> {
        let apps = self
            .apps
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        Ok(apps.get(id).cloned())
    }

    async fn list(&self) -> anyhow::Result<Vec<MarketplaceApp>> {
        let apps = self
            .apps
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        Ok(apps.values().cloned().collect())
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        let mut apps = self
            .apps
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        apps.remove(id);
        Ok(())
    }
}

/// Etcd-backed store using keys `/thiscloud/marketplace/<id>`.
#[derive(Clone)]
pub struct EtcdMarketplaceStore {
    client: EtcdClient,
}

impl EtcdMarketplaceStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(id: &str) -> String {
        format!("/thiscloud/marketplace/{}", id)
    }
}

#[async_trait]
impl MarketplaceStore for EtcdMarketplaceStore {
    async fn put(&self, app: &MarketplaceApp) -> anyhow::Result<()> {
        let json = serde_json::to_string(app)?;
        self.client.put(&Self::key(&app.id), &json).await
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<MarketplaceApp>> {
        match self.client.get(&Self::key(id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<MarketplaceApp>> {
        Err(anyhow::anyhow!(
            "list not supported for EtcdMarketplaceStore yet; use a prefix range"
        ))
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(id)).await
    }
}
