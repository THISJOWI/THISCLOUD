use crate::compute::vm::VmConfig;
use crate::core::EtcdClient;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait VmStore: Send + Sync {
    async fn put(&self, tenant_id: &str, vm: &VmConfig) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<VmConfig>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<VmConfig>>;
    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryVmStore {
    vms: Arc<Mutex<HashMap<String, VmConfig>>>,
}

impl MemoryVmStore {
    fn composite_key(tenant_id: &str, id: &str) -> String {
        format!("{}:{}", tenant_id, id)
    }
}

#[async_trait::async_trait]
impl VmStore for MemoryVmStore {
    async fn put(&self, tenant_id: &str, vm: &VmConfig) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &vm.id);
        self.vms.lock().unwrap().insert(key, vm.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<VmConfig>> {
        let key = Self::composite_key(tenant_id, id);
        Ok(self.vms.lock().unwrap().get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<VmConfig>> {
        let store = self.vms.lock().unwrap();
        if tenant_id.is_empty() {
            // Global admin: return all.
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
        self.vms.lock().unwrap().remove(&key);
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

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/vms/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/vms/")
        }
    }
}

#[async_trait::async_trait]
impl VmStore for EtcdVmStore {
    async fn put(&self, tenant_id: &str, vm: &VmConfig) -> anyhow::Result<()> {
        let json = serde_json::to_string(vm)?;
        self.client.put(&Self::key(tenant_id, &vm.id), &json).await
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<VmConfig>> {
        match self.client.get(&Self::key(tenant_id, id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<VmConfig>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut vms = Vec::new();
        for (_, json) in entries {
            if let Ok(vm) = serde_json::from_str::<VmConfig>(&json) {
                // For global admin (empty tenant_id), filter by the vm's tenant_id.
                // For scoped, include all from this prefix.
                if tenant_id.is_empty() || vm.tenant_id == tenant_id {
                    vms.push(vm);
                }
            }
        }
        Ok(vms)
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, id)).await
    }
}
