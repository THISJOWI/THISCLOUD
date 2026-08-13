use std::sync::atomic::{AtomicBool, Ordering};
use thiscloudd::core::Module;

struct TestModule {
    name: String,
    started: AtomicBool,
}

#[async_trait::async_trait]
impl Module for TestModule {
    fn name(&self) -> &str {
        &self.name
    }

    async fn start(&mut self, _event_bus: &thiscloudd::core::EventBus) -> anyhow::Result<()> {
        self.started.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        self.started.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.started.load(Ordering::SeqCst)
    }
}

#[tokio::test]
async fn test_daemon_creation() {
    let config = thiscloudd::config::ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    assert_eq!(daemon.module_count().await, 6);
}

#[tokio::test]
async fn test_daemon_with_config() {
    let config_content = r#"
[cluster]
name = "test-cluster"
"#;
    let config: thiscloudd::config::ThisCloudConfig = toml::from_str(config_content).unwrap();
    let daemon = thiscloudd::core::Daemon::new(config);

    assert_eq!(daemon.cluster_name(), "test-cluster");
    assert_eq!(daemon.module_count().await, 6);
}

#[tokio::test]
async fn test_daemon_register_module() {
    let config = thiscloudd::config::ThisCloudConfig::default();
    let mut daemon = thiscloudd::core::Daemon::new(config);

    daemon.register_module(Box::new(TestModule {
        name: "test-module".to_string(),
        started: AtomicBool::new(false),
    })).await;

    assert_eq!(daemon.module_count().await, 7);
}

#[tokio::test]
async fn test_daemon_start_stop() {
    let config = thiscloudd::config::ThisCloudConfig::default();
    let mut daemon = thiscloudd::core::Daemon::new(config);

    daemon.register_module(Box::new(TestModule {
        name: "test-module".to_string(),
        started: AtomicBool::new(false),
    }))
    .await;

    daemon.start().await.unwrap();
    daemon.stop().await.unwrap();
}
