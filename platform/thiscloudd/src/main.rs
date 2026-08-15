use anyhow::Result;
use thiscloudd::config::ThisCloudConfig;
use thiscloudd::core::{Daemon, EtcdClient, EtcdManager};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting thiscloudd v{}", env!("CARGO_PKG_VERSION"));

    let config_path = std::path::PathBuf::from("/etc/thiscloud/config.toml");
    let config = if config_path.exists() {
        ThisCloudConfig::load(&config_path)?
    } else {
        tracing::warn!(
            "No config file found at {}, using defaults",
            config_path.display()
        );
        ThisCloudConfig::default()
    };

    // Connect to etcd: embedded single-node (dev/homelab) or the shared
    // multi-master cluster. Falls back to in-memory stores when unreachable.
    let etcd_config = config.cluster.etcd.clone();
    let (_etcd_manager, etcd): (Option<EtcdManager>, Option<EtcdClient>) = if etcd_config.embedded {
        let mut manager = EtcdManager::new(etcd_config.clone());
        manager.start().await?;
        match manager.connect().await {
            Ok(client) => {
                tracing::info!("Connected to embedded etcd");
                (Some(manager), Some(client))
            }
            Err(e) => {
                tracing::warn!(
                    "Embedded etcd unavailable ({:#}) — falling back to in-memory stores",
                    e
                );
                (Some(manager), None)
            }
        }
    } else if !etcd_config.endpoints.is_empty() {
        let mut client = None;
        for endpoint in &etcd_config.endpoints {
            match EtcdClient::connect(endpoint).await {
                Ok(c) => {
                    client = Some(c);
                    break;
                }
                Err(e) => tracing::warn!("etcd endpoint {} unreachable: {e}", endpoint),
            }
        }
        match client {
            Some(c) => {
                let ep = c.endpoints().first().cloned().unwrap_or_default();
                tracing::info!("Connected to etcd at {}", ep);
                (None, Some(c))
            }
            None => {
                tracing::warn!("No etcd endpoint reachable — falling back to in-memory stores");
                (None, None)
            }
        }
    } else {
        tracing::warn!("cluster.etcd.embedded=false with no endpoints — using in-memory stores");
        (None, None)
    };

    let mut daemon = Daemon::new(config, etcd);
    daemon.run().await?;

    Ok(())
}