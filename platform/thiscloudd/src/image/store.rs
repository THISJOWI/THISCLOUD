use crate::core::EtcdClient;
use crate::image::Image;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait ImageStore: Send + Sync {
    async fn put(&self, tenant_id: &str, image: &Image) -> anyhow::Result<()>;
    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<Image>>;
    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<Image>>;
    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()>;
}

#[derive(Debug, Default)]
pub struct MemoryImageStore {
    images: Arc<Mutex<HashMap<String, Image>>>,
}

impl MemoryImageStore {
    fn composite_key(tenant_id: &str, id: &str) -> String {
        format!("{}:{}", tenant_id, id)
    }
}

#[async_trait]
impl ImageStore for MemoryImageStore {
    async fn put(&self, tenant_id: &str, image: &Image) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, &image.id);
        self.images
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?
            .insert(key, image.clone());
        Ok(())
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<Image>> {
        let key = Self::composite_key(tenant_id, id);
        let images = self
            .images
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        Ok(images.get(&key).cloned())
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<Image>> {
        let images = self
            .images
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
        if tenant_id.is_empty() {
            Ok(images.values().cloned().collect())
        } else {
            let prefix = format!("{}:", tenant_id);
            Ok(images
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_, v)| v.clone())
                .collect())
        }
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let key = Self::composite_key(tenant_id, id);
        self.images
            .lock()
            .map_err(|_| anyhow::anyhow!("lock poisoned"))?
            .remove(&key);
        Ok(())
    }
}

#[derive(Clone)]
pub struct EtcdImageStore {
    client: EtcdClient,
}

impl EtcdImageStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(tenant_id: &str, id: &str) -> String {
        format!("/thiscloud/tenants/{tenant_id}/images/{id}")
    }

    fn prefix(tenant_id: &str) -> String {
        if tenant_id.is_empty() {
            "/thiscloud/tenants/".to_string()
        } else {
            format!("/thiscloud/tenants/{tenant_id}/images/")
        }
    }
}

#[async_trait]
impl ImageStore for EtcdImageStore {
    async fn put(&self, tenant_id: &str, image: &Image) -> anyhow::Result<()> {
        let json = serde_json::to_string(image)?;
        self.client.put(&Self::key(tenant_id, &image.id), &json).await
    }

    async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Option<Image>> {
        match self.client.get(&Self::key(tenant_id, id)).await? {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<Image>> {
        let prefix = Self::prefix(tenant_id);
        let entries = self.client.list_prefix(&prefix).await?;
        let mut images = Vec::new();
        for (_, json) in entries {
            if let Ok(image) = serde_json::from_str::<Image>(&json) {
                if tenant_id.is_empty() || image.tenant_id == tenant_id {
                    images.push(image);
                }
            }
        }
        Ok(images)
    }

    async fn delete(&self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        self.client.delete(&Self::key(tenant_id, id)).await
    }
}