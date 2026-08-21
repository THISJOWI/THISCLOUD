use crate::core::Module;
use crate::s3::{S3AccessKey, S3Backend, S3Bucket, S3Store};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

pub struct S3Module {
    backend: Box<dyn S3Backend>,
    store: Box<dyn S3Store>,
    keys: Arc<Mutex<Vec<S3AccessKey>>>,
}

impl S3Module {
    pub fn new(backend: Box<dyn S3Backend>, store: Box<dyn S3Store>) -> Self {
        Self {
            backend,
            store,
            keys: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn create_bucket(&mut self, tenant_id: &str, name: &str) -> anyhow::Result<S3Bucket> {
        if name.is_empty() {
            anyhow::bail!("bucket name is required");
        }
        if self.store.get(tenant_id, name).await?.is_some() {
            anyhow::bail!("bucket '{name}' already exists");
        }
        let mut bucket = S3Bucket::new(name.to_string());
        bucket.tenant_id = tenant_id.to_string();
        self.backend.create_user(tenant_id).await?;
        self.backend.create_bucket(&bucket).await?;
        self.store.put(tenant_id, &bucket).await?;
        tracing::info!("S3 bucket created: {} (tenant {})", bucket.name, tenant_id);
        Ok(bucket)
    }

    pub async fn get_bucket(&self, tenant_id: &str, name: &str) -> anyhow::Result<S3Bucket> {
        self.store
            .get(tenant_id, name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("bucket {name} not found"))
    }

    pub async fn list_buckets(&self, tenant_id: &str) -> anyhow::Result<Vec<S3Bucket>> {
        self.store.list(tenant_id).await
    }

    pub async fn delete_bucket(&mut self, tenant_id: &str, name: &str) -> anyhow::Result<()> {
        let bucket = self.get_bucket(tenant_id, name).await?;
        self.backend.delete_bucket(&bucket).await?;
        self.store.delete(tenant_id, name).await?;
        tracing::info!("S3 bucket deleted: {} (tenant {})", bucket.name, tenant_id);
        Ok(())
    }

    pub async fn issue_credentials(&mut self, tenant_id: &str) -> anyhow::Result<S3AccessKey> {
        self.backend.create_user(tenant_id).await?;
        let mut key = S3AccessKey::new(
            random_hex(20),
            random_hex(40),
            format!("thiscloud-{tenant_id}"),
        );
        key.tenant_id = tenant_id.to_string();
        self.keys
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?
            .push(key.clone());
        tracing::info!("S3 credentials issued for tenant {tenant_id}");
        Ok(key)
    }

    pub async fn list_credentials(&self, tenant_id: &str) -> anyhow::Result<Vec<S3AccessKey>> {
        let keys = self
            .keys
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        if tenant_id.is_empty() {
            Ok(keys.clone())
        } else {
            Ok(keys
                .iter()
                .filter(|k| k.tenant_id == tenant_id)
                .cloned()
                .collect())
        }
    }
}

/// Random lowercase hex string of the requested length (uuid-based, no extra deps).
fn random_hex(len: usize) -> String {
    let mut out = String::new();
    while out.len() < len {
        out.push_str(&uuid::Uuid::new_v4().simple().to_string());
    }
    out.truncate(len);
    out
}

#[async_trait]
impl Module for S3Module {
    fn name(&self) -> &str {
        "s3"
    }

    async fn start(&mut self, _event_bus: &crate::core::EventBus) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}