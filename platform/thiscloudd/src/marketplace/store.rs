use crate::core::EtcdClient;
use crate::marketplace::MarketplaceApp;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait MarketplaceStore: Send + Sync {
    async fn put(&self, tenant_id: &str, app: &MarketplaceApp) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<MarketplaceApp>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<MarketplaceApp>>;
    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()>;
}

#[derive(Debug, Default)]
pub struct MemoryMarketplaceStore {
    apps: Arc<Mutex<HashMap<String, MarketplaceApp>>>,
}

impl MemoryMarketplaceStore {
    fn composite_key(tenant_id: &str, id: &str) -> String {
        format!("{}:{}", tenant_id, id)
    }
}

#[async_trait]
impl MarketplaceStore for MemoryMarketplaceStore {
    async fn put(&self, tenant_id: &str, app: &MarketplaceApp) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &app.id);
        self.apps
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?
            .insert(key, app.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<MarketplaceApp>> {
        let key = Self::composite_key(tenant_id, id);
        let apps = self
            .apps
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        Ok(apps.get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<MarketplaceApp>> {
        let apps = self
            .apps
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        if tenant_id.is_empty() {
            Ok(apps.values().cloned().collect())
        } else {
            let prefix = format!("{}:", tenant_id);
            Ok(apps
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_, v)| v.clone())
                .collect())
        }
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, id);
        self.apps
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?
            .remove(&key);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdMarketplaceStore {
    client: EtcdClient,
}

impl EtcdMarketplaceStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/marketplace/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/marketplace/")
        }
    }
}

#[async_trait]
impl MarketplaceStore for EtcdMarketplaceStore {
    async fn put(&self, tenant_id: &str, app: &MarketplaceApp) -> anyhow::Result<()> {
        let json = serde_json::to_string(app)?;
        self.client.put(&Self::key(tenant_id, &app.id), &json).await
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<MarketplaceApp>> {
        match self.client.get(&Self::key(tenant_id, id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<MarketplaceApp>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut apps = Vec::new();
        for (_, json) in entries {
            if let Ok(app) = serde_json::from_str::<MarketplaceApp>(&json) {
                if tenant_id.is_empty() || app.tenant_id == tenant_id {
                    apps.push(app);
                }
            }
        }
        Ok(apps)
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, id)).await
    }
}
