use crate::core::EtcdClient;
use crate::network::LogicalNetwork;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait NetworkStore: Send + Sync {
    async fn put(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<LogicalNetwork>>;
    async fn list(&self) -> anyhow::Result<Vec<LogicalNetwork>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryNetworkStore {
    networks: Arc<Mutex<HashMap<String, LogicalNetwork>>>,
}

#[async_trait::async_trait]
impl NetworkStore for MemoryNetworkStore {
    async fn put(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.networks
            .lock()
            .unwrap()
            .insert(net.id.clone(), net.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<LogicalNetwork>> {
        Ok(self.networks.lock().unwrap().get(id).cloned())
    }

    async fn list(&self) -> anyhow::Result<Vec<LogicalNetwork>> {
        Ok(self.networks.lock().unwrap().values().cloned().collect())
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.networks.lock().unwrap().remove(id);
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

    fn key(id: &str) -> String {
        format!("/thiscloud/networks/{}", id)
    }
}

#[async_trait::async_trait]
impl NetworkStore for EtcdNetworkStore {
    async fn put(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        let json = serde_json::to_string(net)?;
        self.client.put(&Self::key(&net.id), &json).await
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<LogicalNetwork>> {
        match self.client.get(&Self::key(id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<LogicalNetwork>> {
        Err(anyhow::anyhow!(
            "list not supported for EtcdNetworkStore yet; use a prefix range"
        ))
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(id)).await
    }
}
