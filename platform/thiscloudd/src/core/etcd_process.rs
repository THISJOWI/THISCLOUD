use crate::config::EtcdConfig;
use std::path::PathBuf;
use tokio::process::Child;
use tokio::time::{sleep, Duration};

/// Manages the lifecycle of an embedded etcd process and provides a
/// client for cluster state operations.
pub struct EtcdManager {
    config: EtcdConfig,
    child: Option<Child>,
}

impl EtcdManager {
    pub fn new(config: EtcdConfig) -> Self {
        Self {
            config,
            child: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.config.embedded
    }

    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }

    /// Start the local etcd process. Only effective when `embed` is true.
    pub async fn start(&mut self) -> anyhow::Result<()> {
        if !self.config.embedded {
            return Ok(());
        }

        let data_dir = PathBuf::from(&self.config.data_dir);
        tokio::fs::create_dir_all(&data_dir).await?;

        let child = tokio::process::Command::new("etcd")
            .arg("--name")
            .arg("thiscloud")
            .arg("--listen-client-urls")
            .arg(format!("http://127.0.0.1:{}", self.config.port))
            .arg("--advertise-client-urls")
            .arg(format!("http://127.0.0.1:{}", self.config.port))
            .arg("--listen-peer-urls")
            .arg(format!("http://127.0.0.1:{}", self.config.peer_port))
            .arg("--initial-advertise-peer-urls")
            .arg(format!("http://127.0.0.1:{}", self.config.peer_port))
            .arg("--data-dir")
            .arg(data_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        self.child = Some(child);
        tracing::info!("etcd embedded started on port {}", self.config.port);
        Ok(())
    }

    pub async fn connect(&self) -> anyhow::Result<super::etcd::EtcdClient> {
        let endpoint = format!("http://127.0.0.1:{}", self.config.port);
        for _ in 0..20 {
            if let Ok(client) = super::etcd::EtcdClient::connect(&endpoint).await {
                // Verify the connection is actually usable before returning.
                if client.get("/thiscloud/.ready").await.is_ok() {
                    return Ok(client);
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        anyhow::bail!("etcd did not become ready on {}", endpoint)
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
        tracing::info!("etcd embedded stopped");
    }
}

impl Drop for EtcdManager {
    fn drop(&mut self) {
        self.stop();
    }
}
