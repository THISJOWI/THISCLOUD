use crate::image::Image;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::process::Command;

#[async_trait]
pub trait ImageBackend: Send + Sync {
    /// Import the image artifact (download/verify) so it is ready to boot.
    async fn import(&self, image: &Image) -> anyhow::Result<()>;
    /// Persist an uploaded artifact (raw bytes) so it is ready to boot.
    async fn store(&self, image: &Image, data: &[u8]) -> anyhow::Result<()>;
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

    async fn store(&self, image: &Image, _data: &[u8]) -> anyhow::Result<()> {
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

    async fn store(&self, image: &Image, data: &[u8]) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.images_dir)
            .map_err(|e| anyhow::anyhow!("failed to create images dir: {e}"))?;

        let dest = self.artifact_path(image);
        std::fs::write(&dest, data)
            .map_err(|e| anyhow::anyhow!("failed to write artifact: {e}"))?;

        if !image.sha256.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(data);
            let digest = hasher.finalize();
            let got = digest.iter().map(|b| format!("{b:02x}")).collect::<String>();
            if got != image.sha256 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_image(format: crate::image::ImageFormat, sha256: &str) -> Image {
        Image {
            id: "test-img-1".to_string(),
            name: "test".to_string(),
            source: String::new(),
            sha256: sha256.to_string(),
            size_bytes: 0,
            format,
            os_family: crate::image::OsFamily::Generic,
            version: "1".to_string(),
            template: false,
            status: crate::image::ImageStatus::Available,
            tenant_id: String::new(),
        }
    }

    #[tokio::test]
    async fn local_store_writes_artifact_file() {
        let dir = std::env::temp_dir().join(format!("tc-img-test-{}", uuid::Uuid::new_v4()));
        let backend = LocalImageBackend::new(dir.to_string_lossy().to_string());
        let data = b"hello thiscloud artifact";

        backend
            .store(&test_image(crate::image::ImageFormat::Raw, ""), data)
            .await
            .unwrap();

        let path = dir.join("test-img-1.img");
        assert_eq!(std::fs::read(&path).unwrap(), data);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn local_store_verifies_sha256_when_provided() {
        let dir = std::env::temp_dir().join(format!("tc-img-test-{}", uuid::Uuid::new_v4()));
        let backend = LocalImageBackend::new(dir.to_string_lossy().to_string());
        let data = b"checksum me";
        let digest = {
            let mut hasher = Sha256::new();
            hasher.update(data);
            hasher.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>()
        };

        backend
            .store(&test_image(crate::image::ImageFormat::Iso, &digest), data)
            .await
            .unwrap();
        let path = dir.join("test-img-1.iso");
        assert!(path.exists());

        let bad = test_image(crate::image::ImageFormat::Iso, &"0".repeat(64));
        assert!(backend.store(&bad, data).await.is_err());
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn mock_store_accepts_upload() {
        let backend = MockImageBackend::default();
        backend
            .store(&test_image(crate::image::ImageFormat::Qcow2, ""), b"data")
            .await
            .unwrap();
        assert!(lock_set(&backend.imported).unwrap().contains("test-img-1"));
    }
}