use crate::marketplace::MarketplaceApp;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::process::Command;

#[async_trait]
pub trait MarketplaceBackend: Send + Sync {
    async fn install(&self, app: &MarketplaceApp) -> anyhow::Result<()>;
    async fn uninstall(&self, app: &MarketplaceApp) -> anyhow::Result<()>;
    async fn exists(&self, name: &str) -> anyhow::Result<bool>;
}

fn lock_set(
    set: &Mutex<HashSet<String>>,
) -> anyhow::Result<std::sync::MutexGuard<'_, HashSet<String>>> {
    set.lock().map_err(|_| anyhow::anyhow!("lock poisoned"))
}

/// In-memory backend used by tests and macOS dev.
#[derive(Debug, Default)]
pub struct MockMarketplaceBackend {
    installed: Arc<Mutex<HashSet<String>>>,
}

#[async_trait]
impl MarketplaceBackend for MockMarketplaceBackend {
    async fn install(&self, app: &MarketplaceApp) -> anyhow::Result<()> {
        lock_set(&self.installed)?.insert(app.name.clone());
        Ok(())
    }

    async fn uninstall(&self, app: &MarketplaceApp) -> anyhow::Result<()> {
        lock_set(&self.installed)?.remove(&app.name);
        Ok(())
    }

    async fn exists(&self, name: &str) -> anyhow::Result<bool> {
        Ok(lock_set(&self.installed)?.contains(name))
    }
}

/// Backend that shells out to `docker` for images and to a local TurboKit
/// binary for ISOs/cloud-init/TurboKit artifacts. Only runs inside the ISO;
/// on dev the mock backend is used.
#[derive(Debug, Default)]
pub struct DockerHubBackend {
    installed: Arc<Mutex<HashSet<String>>>,
}

impl DockerHubBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn install_command(app: &MarketplaceApp) -> Vec<String> {
        match app.app_type {
            crate::marketplace::AppType::DockerImage => {
                vec!["docker".to_string(), "pull".to_string(), app.source.clone()]
            }
            _ => vec![
                "turbokit".to_string(),
                "install".to_string(),
                app.source.clone(),
            ],
        }
    }
}

#[async_trait]
impl MarketplaceBackend for DockerHubBackend {
    async fn install(&self, app: &MarketplaceApp) -> anyhow::Result<()> {
        let cmd = Self::install_command(app);
        tracing::info!("marketplace install command: {:?}", cmd);

        let status = Command::new(&cmd[0])
            .args(&cmd[1..])
            .status()
            .await
            .map_err(|e| anyhow::anyhow!("failed to run install command: {}", e))?;

        if !status.success() {
            anyhow::bail!("install command failed with status: {}", status);
        }

        lock_set(&self.installed)?.insert(app.source.clone());
        Ok(())
    }

    async fn uninstall(&self, app: &MarketplaceApp) -> anyhow::Result<()> {
        let status = Command::new("docker")
            .args(["rmi", &app.source])
            .status()
            .await
            .map_err(|e| anyhow::anyhow!("failed to run uninstall command: {}", e))?;

        if !status.success() {
            anyhow::bail!("uninstall command failed with status: {}", status);
        }

        lock_set(&self.installed)?.remove(&app.source);
        Ok(())
    }

    async fn exists(&self, name: &str) -> anyhow::Result<bool> {
        Ok(lock_set(&self.installed)?.contains(name))
    }
}
