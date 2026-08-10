use crate::core::EtcdClient;
use crate::storage::StoragePool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait StorageStore: Send + Sync {
    async fn put(&self, pool: &StoragePool) -> anyhow::Result<()>;
    async fn get(&self, name: &str) -> anyhow::Result<Option<StoragePool>>;
    async fn list(&self) -> anyhow::Result<Vec<StoragePool>>;
    async fn delete(&self, name: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryStorageStore {
    pools: Arc<Mutex<HashMap<String, StoragePool>>>,
}

#[async_trait::async_trait]
impl StorageStore for MemoryStorageStore {
    async fn put(&self, pool: &StoragePool) -> anyhow::Result<()> {
        self.pools
            .lock()
            .unwrap()
            .insert(pool.name.clone(), pool.clone());
        Ok(())
    }

    async fn get(&self, name: &str) -> anyhow::Result<Option<StoragePool>> {
        Ok(self.pools.lock().unwrap().get(name).cloned())
    }

    async fn list(&self) -> anyhow::Result<Vec<StoragePool>> {
        Ok(self.pools.lock().unwrap().values().cloned().collect())
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        self.pools.lock().unwrap().remove(name);
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

    fn key(name: &str) -> String {
        format!("/thiscloud/storage/pools/{}", name)
    }
}

#[async_trait::async_trait]
impl StorageStore for EtcdStorageStore {
    async fn put(&self, pool: &StoragePool) -> anyhow::Result<()> {
        let json = serde_json::to_string(pool)?;
        self.client.put(&Self::key(&pool.name), &json).await
    }

    async fn get(&self, name: &str) -> anyhow::Result<Option<StoragePool>> {
        match self.client.get(&Self::key(name)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<StoragePool>> {
        Err(anyhow::anyhow!(
            "list not supported for EtcdStorageStore yet; use a prefix range"
        ))
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(name)).await
    }
}
