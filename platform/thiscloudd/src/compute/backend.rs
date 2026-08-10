use crate::compute::vm::{VmConfig, VmStatus};
use tokio::process::Command;

#[async_trait::async_trait]
pub trait HypervisorBackend: Send + Sync {
    async fn spawn(&self, vm: &VmConfig) -> anyhow::Result<()>;
    async fn stop(&self, vm: &VmConfig) -> anyhow::Result<()>;
    async fn status(&self, vm: &VmConfig) -> anyhow::Result<VmStatus>;
}

pub struct MockHypervisor {
    running: std::sync::Mutex<Vec<String>>,
}

impl MockHypervisor {
    pub fn new() -> Self {
        Self {
            running: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Default for MockHypervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl HypervisorBackend for MockHypervisor {
    async fn spawn(&self, vm: &VmConfig) -> anyhow::Result<()> {
        self.running.lock().unwrap().push(vm.id.clone());
        Ok(())
    }

    async fn stop(&self, vm: &VmConfig) -> anyhow::Result<()> {
        self.running.lock().unwrap().retain(|id| id != &vm.id);
        Ok(())
    }

    async fn status(&self, vm: &VmConfig) -> anyhow::Result<VmStatus> {
        Ok(if self.running.lock().unwrap().contains(&vm.id) {
            VmStatus::Running
        } else {
            VmStatus::Stopped
        })
    }
}

pub struct CloudHypervisor;

impl CloudHypervisor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CloudHypervisor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl HypervisorBackend for CloudHypervisor {
    async fn spawn(&self, vm: &VmConfig) -> anyhow::Result<()> {
        let mut cmd = Command::new("cloud-hypervisor");
        cmd.arg("--cpus")
            .arg(format!("boot={}", vm.cpus))
            .arg("--memory")
            .arg(format!("size={}M", vm.memory_mb))
            .arg("--disk")
            .arg(format!("path={}", vm.disk_path));
        if !vm.kernel.is_empty() {
            cmd.arg("--kernel").arg(&vm.kernel);
        }
        if !vm.kernel_args.is_empty() {
            cmd.arg("--cmdline").arg(&vm.kernel_args);
        }
        for net in &vm.networks {
            cmd.arg("--net").arg(format!("tap={}", net));
        }
        let status = cmd.status().await?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("cloud-hypervisor spawn failed: {:?}", status)
        }
    }

    async fn stop(&self, vm: &VmConfig) -> anyhow::Result<()> {
        Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(format!("/tmp/thiscloud-{}.sock", vm.id))
            .arg("shutdown")
            .status()
            .await?;
        Ok(())
    }

    async fn status(&self, vm: &VmConfig) -> anyhow::Result<VmStatus> {
        let output = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(format!("/tmp/thiscloud-{}.sock", vm.id))
            .arg("info")
            .output()
            .await?;
        Ok(if output.status.success() {
            VmStatus::Running
        } else {
            VmStatus::Stopped
        })
    }
}
