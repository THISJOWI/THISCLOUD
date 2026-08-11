use crate::core::EtcdClient;
use crate::network::LogicalNetwork;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait NetworkStore: Send + Sync {
    async fn put(&self, tenant_id: &str, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<LogicalNetwork>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<LogicalNetwork>>;
    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryNetworkStore {
    networks: Arc<Mutex<HashMap<String, LogicalNetwork>>>,
}

impl MemoryNetworkStore {
    fn composite_key(tenant_id: &str, id: &str) -> String {
        format!("{}:{}", tenant_id, id)
    }
}

#[async_trait::async_trait]
impl NetworkStore for MemoryNetworkStore {
    async fn put(&self, tenant_id: &str, net: &LogicalNetwork) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &net.id);
        self.networks.lock().unwrap().insert(key, net.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<LogicalNetwork>> {
        let key = Self::composite_key(tenant_id, id);
        Ok(self.networks.lock().unwrap().get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<LogicalNetwork>> {
        let store = self.networks.lock().unwrap();
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

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, id);
        self.networks.lock().unwrap().remove(&key);
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

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/networks/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/networks/")
        }
    }
}

#[async_trait::async_trait]
impl NetworkStore for EtcdNetworkStore {
    async fn put(&self, tenant_id: &str, net: &LogicalNetwork) -> anyhow::Result<()> {
        let json = serde_json::to_string(net)?;
        self.client.put(&Self::key(tenant_id, &net.id), &json).await
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<LogicalNetwork>> {
        match self.client.get(&Self::key(tenant_id, id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<LogicalNetwork>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut nets = Vec::new();
        for (_, json) in entries {
            if let Ok(net) = serde_json::from_str::<LogicalNetwork>(&json) {
                if tenant_id.is_empty() || net.tenant_id == tenant_id {
                    nets.push(net);
                }
            }
        }
        Ok(nets)
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, id)).await
    }
}
