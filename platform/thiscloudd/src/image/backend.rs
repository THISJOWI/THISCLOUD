use crate::image::Image;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::process::Command;

#[async_trait]
pub trait ImageBackend: Send + Sync {
    /// Import the image artifact (download/verify) so it is ready to boot.
    async fn import(&self, image: &Image) -> anyhow::Result<()>;
    /// Remove the artifact from disk.
    async fn remove(&self, image: &Image) -> anyhow::Result<()>;
}

fn lock_set(
    set: &Mutex<HashSet<String>>,
) -> anyhow::Result<std::sync::MutexGuard<'_, HashSet<String>>> {
    set.lock().map_err(|_| anyhow::anyhow!("lock poisoned"))
}

/// In-memory backend used by tests and macOS dev.
#[derive(Debug, Default)]
pub struct MockImageBackend {
    imported: Arc<Mutex<HashSet<String>>>,
}

#[async_trait]
impl ImageBackend for MockImageBackend {
    async fn import(&self, image: &Image) -> anyhow::Result<()> {
        lock_set(&self.imported)?.insert(image.id.clone());
        Ok(())
    }

    async fn remove(&self, image: &Image) -> anyhow::Result<()> {
        lock_set(&self.imported)?.remove(&image.id);
        Ok(())
    }
}

/// Backend that downloads artifacts into a local images directory with
/// `curl` and verifies the SHA-256 checksum when provided. Only runs inside
/// the ISO; on dev the mock backend is used.
#[derive(Debug)]
pub struct LocalImageBackend {
    images_dir: String,
}

impl LocalImageBackend {
    pub fn new(images_dir: String) -> Self {
        Self { images_dir }
    }

    fn artifact_path(&self, image: &Image) -> String {
        let ext = match image.format {
            crate::image::ImageFormat::Qcow2 => "qcow2",
            crate::image::ImageFormat::Iso => "iso",
            crate::image::ImageFormat::Raw => "img",
            crate::image::ImageFormat::CloudInit => "cfg",
        };
        format!("{}/{}.{}", self.images_dir, image.id, ext)
    }
}

#[async_trait]
impl ImageBackend for LocalImageBackend {
    async fn import(&self, image: &Image) -> anyhow::Result<()> {
        if image.format == crate::image::ImageFormat::CloudInit {
            // Cloud-init profiles are not fetched; nothing to download.
            return Ok(());
        }
        if image.source.is_empty() {
            anyhow::bail!("image source is empty");
        }

        std::fs::create_dir_all(&self.images_dir)
            .map_err(|e| anyhow::anyhow!("failed to create images dir: {e}"))?;

        let dest = self.artifact_path(image);
        let status = Command::new("curl")
            .args(["-fsSL", "-o", &dest, &image.source])
            .status()
            .await
            .map_err(|e| anyhow::anyhow!("failed to run curl: {e}"))?;
        if !status.success() {
            anyhow::bail!("curl failed with status: {status}");
        }

        if !image.sha256.is_empty() {
            let check = Command::new("sha256sum")
                .args([&dest])
                .output()
                .await
                .map_err(|e| anyhow::anyhow!("failed to run sha256sum: {e}"))?;
            let checksum = String::from_utf8_lossy(&check.stdout);
            if !checksum.starts_with(&image.sha256) {
                let _ = std::fs::remove_file(&dest);
                anyhow::bail!("checksum mismatch for image {}", image.name);
            }
        }
        Ok(())
    }

    async fn remove(&self, image: &Image) -> anyhow::Result<()> {
        let path = self.artifact_path(image);
        if std::path::Path::new(&path).exists() {
            std::fs::remove_file(&path)
                .map_err(|e| anyhow::anyhow!("failed to remove artifact: {e}"))?;
        }
        Ok(())
    }
}