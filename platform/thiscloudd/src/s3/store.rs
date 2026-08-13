use crate::core::EtcdClient;
use crate::s3::S3Bucket;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait S3Store: Send + Sync {
    async fn put(&self, tenant_id: &str, bucket: &S3Bucket) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, name: &str) -> anyhow::Result<Option<S3Bucket>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<S3Bucket>>;
    async fn delete(&self, tenant_id: &str, name: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryS3Store {
    buckets: Arc<Mutex<HashMap<String, S3Bucket>>>,
}

impl MemoryS3Store {
    fn composite_key(tenant_id: &str, name: &str) -> String {
        format!("{}:{}", tenant_id, name)
    }
}

#[async_trait]
impl S3Store for MemoryS3Store {
    async fn put(&self, tenant_id: &str, bucket: &S3Bucket) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &bucket.name);
        self.buckets.lock().unwrap().insert(key, bucket.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, name: &str) -> anyhow::Result<Option<S3Bucket>> {
        let key = Self::composite_key(tenant_id, name);
        Ok(self.buckets.lock().unwrap().get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<S3Bucket>> {
        let store = self.buckets.lock().unwrap();
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
        self.buckets.lock().unwrap().remove(&key);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdS3Store {
    client: EtcdClient,
}

impl EtcdS3Store {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/s3/buckets/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/s3/buckets/")
        }
    }
}

#[async_trait]
impl S3Store for EtcdS3Store {
    async fn put(&self, tenant_id: &str, bucket: &S3Bucket) -> anyhow::Result<()> {
        let json = serde_json::to_string(bucket)?;
        self.client
            .put(&Self::key(tenant_id, &bucket.id), &json)
            .await
    }

    async fn get(&self, tenant_id: &str, name: &str) -> anyhow::Result<Option<S3Bucket>> {
        let entries = self.client.list_prefix(&Self::prefix(tenant_id)).await?;
        for (_, json) in entries {
            if let Ok(bucket) = serde_json::from_str::<S3Bucket>(&json) {
                if bucket.name == name {
                    return Ok(Some(bucket));
                }
            }
        }
        Ok(None)
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<S3Bucket>> {
        let entries = self.client.list_prefix(&Self::prefix(tenant_id)).await?;
        let mut buckets = Vec::new();
        for (_, json) in entries {
            if let Ok(bucket) = serde_json::from_str::<S3Bucket>(&json) {
                if tenant_id.is_empty() || bucket.tenant_id == tenant_id {
                    buckets.push(bucket);
                }
            }
        }
        Ok(buckets)
    }

    async fn delete(&self, tenant_id: &str, name: &str) -> anyhow::Result<()> {
        let entries = self.client.list_prefix(&Self::prefix(tenant_id)).await?;
        for (key, json) in entries {
            if let Ok(bucket) = serde_json::from_str::<S3Bucket>(&json) {
                if bucket.name == name {
                    return self.client.delete(&key).await;
                }
            }
        }
        Ok(())
    }
}