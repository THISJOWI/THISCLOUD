use crate::core::EtcdClient;
use crate::quota::model::TenantQuota;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait QuotaStore: Send + Sync {
    async fn get(&self, tenant_id: &str) -> anyhow::Result<Option<TenantQuota>>;
    async fn set(&self, quota: &TenantQuota) -> anyhow::Result<()>;
    async fn delete(&self, tenant_id: &str) -> anyhow::Result<()>;
    async fn list(&self) -> anyhow::Result<Vec<TenantQuota>>;
}

/// In-memory quota store (default; used in dev and tests).
#[derive(Default)]
pub struct MemoryQuotaStore {
    quotas: Arc<Mutex<HashMap<String, TenantQuota>>>,
}

impl MemoryQuotaStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl QuotaStore for MemoryQuotaStore {
    async fn get(&self, tenant_id: &str) -> anyhow::Result<Option<TenantQuota>> {
        Ok(self.quotas.lock().unwrap().get(tenant_id).cloned())
    }

    async fn set(&self, quota: &TenantQuota) -> anyhow::Result<()> {
        self.quotas
            .lock()
            .unwrap()
            .insert(quota.tenant_id.clone(), quota.clone());
        Ok(())
    }

    async fn delete(&self, tenant_id: &str) -> anyhow::Result<()> {
        self.quotas.lock().unwrap().remove(tenant_id);
        Ok(())
    }

    async fn list(&self) -> anyhow::Result<Vec<TenantQuota>> {
        Ok(self.quotas.lock().unwrap().values().cloned().collect())
    }
}

/// Quota store persisted in etcd. Keys live under `/thiscloud/quotas/`.
#[derive(Clone)]
pub struct EtcdQuotaStore {
    client: EtcdClient,
}

impl EtcdQuotaStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str) -> String {
        format!("/thiscloud/quotas/{tenant_id}")
    }
}

#[async_trait::async_trait]
impl QuotaStore for EtcdQuotaStore {
    async fn get(&self, tenant_id: &str) -> anyhow::Result<Option<TenantQuota>> {
        match self.client.get(&Self::key(tenant_id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn set(&self, quota: &TenantQuota) -> anyhow::Result<()> {
        let json = serde_json::to_string(quota)?;
        self.client.put(&Self::key(&quota.tenant_id), &json).await
    }

    async fn delete(&self, tenant_id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id)).await
    }

    async fn list(&self) -> anyhow::Result<Vec<TenantQuota>> {
        let entries = self.client.list_prefix("/thiscloud/quotas/").await?;
        let mut quotas = Vec::new();
        for (_, json) in entries {
            if let Ok(q) = serde_json::from_str::<TenantQuota>(&json) {
                quotas.push(q);
            }
        }
        Ok(quotas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_store_roundtrip() {
        let store = MemoryQuotaStore::new();
        assert!(store.get("t1").await.unwrap().is_none());

        let q = TenantQuota {
            tenant_id: "t1".into(),
            max_cpus: 8,
            ..TenantQuota::unlimited("t1")
        };
        store.set(&q).await.unwrap();
        assert_eq!(store.get("t1").await.unwrap(), Some(q));
        assert_eq!(store.list().await.unwrap().len(), 1);

        store.delete("t1").await.unwrap();
        assert!(store.get("t1").await.unwrap().is_none());
    }
}
