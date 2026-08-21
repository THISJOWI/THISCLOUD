use crate::core::EtcdClient;
use crate::storage::StoragePool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait StorageStore: Send + Sync {
    async fn put(&self, tenant_id: &str, pool: &StoragePool) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, name: &str) -> anyhow::Result<Option<StoragePool>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<StoragePool>>;
    async fn delete(&self, tenant_id: &str, name: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryStorageStore {
    pools: Arc<Mutex<HashMap<String, StoragePool>>>,
}

impl MemoryStorageStore {
    fn composite_key(tenant_id: &str, name: &str) -> String {
        format!("{}:{}", tenant_id, name)
    }
}

#[async_trait::async_trait]
impl StorageStore for MemoryStorageStore {
    async fn put(&self, tenant_id: &str, pool: &StoragePool) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &pool.name);
        self.pools.lock().unwrap().insert(key, pool.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, name: &str) -> anyhow::Result<Option<StoragePool>> {
        let key = Self::composite_key(tenant_id, name);
        Ok(self.pools.lock().unwrap().get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<StoragePool>> {
        let store = self.pools.lock().unwrap();
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

    async fn delete(&self, tenant_id: &str, name: &str) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, name);
        self.pools.lock().unwrap().remove(&key);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdStorageStore {
    client: EtcdClient,
}

impl EtcdStorageStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str, name: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/storage/pools/{name}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/storage/pools/")
        }
    }
}

#[async_trait::async_trait]
impl StorageStore for EtcdStorageStore {
    async fn put(&self, tenant_id: &str, pool: &StoragePool) -> anyhow::Result<()> {
        let json = serde_json::to_string(pool)?;
        self.client
            .put(&Self::key(tenant_id, &pool.name), &json)
            .await
    }

    async fn get(&self, tenant_id: &str, name: &str) -> anyhow::Result<Option<StoragePool>> {
        match self.client.get(&Self::key(tenant_id, name)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<StoragePool>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut pools = Vec::new();
        for (_, json) in entries {
            if let Ok(pool) = serde_json::from_str::<StoragePool>(&json) {
                if tenant_id.is_empty() || pool.tenant_id == tenant_id {
                    pools.push(pool);
                }
            }
        }
        Ok(pools)
    }

    async fn delete(&self, tenant_id: &str, name: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, name)).await
    }
}
