use crate::core::Module;
use crate::image::{Image, ImageBackend, ImageStatus, ImageStore};
use async_trait::async_trait;

pub struct ImageModule {
    backend: Box<dyn ImageBackend>,
    store: Box<dyn ImageStore>,
}

impl ImageModule {
    pub fn new(backend: Box<dyn ImageBackend>, store: Box<dyn ImageStore>) -> Self {
        Self { backend, store }
    }

    /// Register an image and import its artifact.
    pub async fn register(&mut self, tenant_id: &str, image: &mut Image) -> anyhow::Result<Image> {
        if image.name.is_empty() {
            anyhow::bail!("image name is required");
        }
        if image.format == crate::image::ImageFormat::CloudInit && image.source.is_empty() {
            anyhow::bail!("cloud-init image requires a source");
        }
        if image.id.is_empty() {
            image.id = uuid::Uuid::new_v4().to_string();
        }
        image.tenant_id = tenant_id.to_string();
        image.status = ImageStatus::Available;
        self.backend.import(image).await?;
        self.store.put(tenant_id, image).await?;
        Ok(image.clone())
    }

    pub async fn remove(&mut self, tenant_id: &str, id: &str) -> anyhow::Result<()> {
        let image = self
            .store
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("image {id} not found"))?;
        let _ = self.backend.remove(&image).await;
        self.store.delete(tenant_id, id).await
    }

    pub async fn get(&self, tenant_id: &str, id: &str) -> anyhow::Result<Image> {
        self.store
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("image {id} not found"))
    }

    pub async fn get_by_name(&self, tenant_id: &str, name: &str) -> anyhow::Result<Option<Image>> {
        let images = self.store.list(tenant_id).await?;
        Ok(images.into_iter().find(|i| i.name == name))
    }

    pub async fn set_template(&mut self, tenant_id: &str, id: &str, template: bool) -> anyhow::Result<Image> {
        let mut image = self
            .store
            .get(tenant_id, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("image {id} not found"))?;
        image.template = template;
        self.store.put(tenant_id, &image).await?;
        Ok(image)
    }

    pub async fn list(&self, tenant_id: &str) -> anyhow::Result<Vec<Image>> {
        self.store.list(tenant_id).await
    }
}

#[async_trait]
impl Module for ImageModule {
    fn name(&self) -> &str {
        "image"
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