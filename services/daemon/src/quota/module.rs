use crate::quota::model::{ResourceDelta, TenantQuota};
use crate::quota::store::{MemoryQuotaStore, QuotaStore};

/// Business logic for tenant quotas.
pub struct QuotaModule {
    store: Box<dyn QuotaStore>,
}

impl QuotaModule {
    pub fn new(store: Box<dyn QuotaStore>) -> Self {
        Self { store }
    }

    pub fn with_memory_store() -> Self {
        Self::new(Box::new(MemoryQuotaStore::new()))
    }

    pub async fn get(&self, tenant_id: &str) -> anyhow::Result<TenantQuota> {
        Ok(self
            .store
            .get(tenant_id)
            .await?
            .unwrap_or_else(|| TenantQuota::unlimited(tenant_id)))
    }

    pub async fn set(&self, quota: TenantQuota) -> anyhow::Result<()> {
        self.store.set(&quota).await
    }

    pub async fn delete(&self, tenant_id: &str) -> anyhow::Result<()> {
        self.store.delete(tenant_id).await
    }

    pub async fn list(&self) -> anyhow::Result<Vec<TenantQuota>> {
        self.store.list().await
    }

    /// Check that `usage` (current totals) is within the tenant quota.
    /// Returns Ok(()) or an error describing what exceeds the limit.
    pub async fn check(&self, tenant_id: &str, usage: &ResourceDelta) -> anyhow::Result<()> {
        let quota = self.get(tenant_id).await?;
        if quota.exceeds(usage) {
            anyhow::bail!(
                "tenant {tenant_id} exceeds quota: cpus {}/{}, mem {}/{}MB, vms {}/{}, storage {}/{}GB, networks {}/{}",
                usage.cpus,
                quota.max_cpus,
                usage.memory_mb,
                quota.max_memory_mb,
                usage.vms,
                quota.max_vms,
                usage.storage_gb,
                quota.max_storage_gb,
                usage.networks,
                quota.max_networks,
            );
        }
        Ok(())
    }
}
