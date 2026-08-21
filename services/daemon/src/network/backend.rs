use crate::network::LogicalNetwork;
use std::collections::HashSet;
use std::sync::Mutex;

#[async_trait::async_trait]
pub trait NetworkBackend: Send + Sync {
    async fn create(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn delete(&self, net: &LogicalNetwork) -> anyhow::Result<()>;
    async fn exists(&self, id: &str) -> anyhow::Result<bool>;
}

#[derive(Default)]
pub struct MockNetworkBackend {
    ids: Mutex<HashSet<String>>,
}

impl MockNetworkBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl NetworkBackend for MockNetworkBackend {
    async fn create(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.ids.lock().unwrap().insert(net.id.clone());
        Ok(())
    }

    async fn delete(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.ids.lock().unwrap().remove(&net.id);
        Ok(())
    }

    async fn exists(&self, id: &str) -> anyhow::Result<bool> {
        Ok(self.ids.lock().unwrap().contains(id))
    }
}

pub struct OvnNetworkBackend;

impl Default for OvnNetworkBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl OvnNetworkBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn create_command(&self, net: &LogicalNetwork) -> Vec<String> {
        vec![
            "ovn-nbctl".to_string(),
            "ls-add".to_string(),
            net.id.clone(),
        ]
    }

    pub fn set_subnet_command(&self, net: &LogicalNetwork) -> Vec<String> {
        vec![
            "ovn-nbctl".to_string(),
            "set".to_string(),
            "Logical_Switch".to_string(),
            net.id.clone(),
            format!("other_config:subnet={}", net.cidr),
        ]
    }

    pub fn delete_command(&self, net: &LogicalNetwork) -> Vec<String> {
        vec![
            "ovn-nbctl".to_string(),
            "ls-del".to_string(),
            net.id.clone(),
        ]
    }
}

#[async_trait::async_trait]
impl NetworkBackend for OvnNetworkBackend {
    async fn create(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.run(&self.create_command(net)).await?;
        self.run(&self.set_subnet_command(net)).await?;
        Ok(())
    }

    async fn delete(&self, net: &LogicalNetwork) -> anyhow::Result<()> {
        self.run(&self.delete_command(net)).await?;
        Ok(())
    }

    async fn exists(&self, id: &str) -> anyhow::Result<bool> {
        let output = tokio::process::Command::new("ovn-nbctl")
            .arg("ls-list")
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().any(|l| l.contains(id)))
    }
}

impl OvnNetworkBackend {
    async fn run(&self, cmd: &[String]) -> anyhow::Result<()> {
        let status = tokio::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("ovn-nbctl command failed: {:?}", status)
        }
    }
}
