use crate::compute::vm::VmConfig;
use crate::core::EtcdClient;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait VmStore: Send + Sync {
    async fn put(&self, vm: &VmConfig) -> anyhow::Result<()>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<VmConfig>>;
    async fn list(&self) -> anyhow::Result<Vec<VmConfig>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryVmStore {
    vms: Arc<Mutex<HashMap<String, VmConfig>>>,
}

#[async_trait::async_trait]
impl VmStore for MemoryVmStore {
    async fn put(&self, vm: &VmConfig) -> anyhow::Result<()> {
        self.vms.lock().unwrap().insert(vm.id.clone(), vm.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<VmConfig>> {
        Ok(self.vms.lock().unwrap().get(id).cloned())
    }

    async fn list(&self) -> anyhow::Result<Vec<VmConfig>> {
        Ok(self.vms.lock().unwrap().values().cloned().collect())
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.vms.lock().unwrap().remove(id);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdVmStore {
    client: EtcdClient,
}

impl EtcdVmStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(id: &str) -> String {
        format!("/thiscloud/vms/{}", id)
    }
}

#[async_trait::async_trait]
impl VmStore for EtcdVmStore {
    async fn put(&self, vm: &VmConfig) -> anyhow::Result<()> {
        let json = serde_json::to_string(vm)?;
        self.client.put(&Self::key(&vm.id), &json).await
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<VmConfig>> {
        match self.client.get(&Self::key(id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> anyhow::Result<Vec<VmConfig>> {
        // List all VM keys is not supported by EtcdClient directly;
        // This is a best-effort placeholder using known key prefix.
        Err(anyhow::anyhow!(
            "list not supported for EtcdVmStore yet; use a prefix range"
        ))
    }

    async fn delete(&self, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(id)).await
    }
}
