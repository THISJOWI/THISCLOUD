use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting thiscloudd v{}", env!("CARGO_PKG_VERSION"));

    let config_path = std::path::PathBuf::from("/etc/thiscloud/config.toml");
    let config = if config_path.exists() {
        thiscloudd::config::ThisCloudConfig::load(&config_path)?
    } else {
        tracing::warn!(
            "No config file found at {}, using defaults",
            config_path.display()
        );
        thiscloudd::config::ThisCloudConfig::default()
    };

    let mut daemon = thiscloudd::core::Daemon::new(config);
    daemon.run().await?;

    Ok(())
}
