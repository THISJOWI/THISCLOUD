use crate::compute::vm::{DiskConfig, Snapshot, VmConfig, VmStatus};
use tokio::process::Command;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait HypervisorBackend: Send + Sync {
    async fn spawn(&self, vm: &VmConfig) -> anyhow::Result<()>;
    async fn stop(&self, vm: &VmConfig) -> anyhow::Result<()>;
    async fn status(&self, vm: &VmConfig) -> anyhow::Result<VmStatus>;
    /// Take a snapshot of the VM; returns the snapshot descriptor to persist.
    async fn snapshot(&self, vm: &VmConfig, name: &str) -> anyhow::Result<Snapshot>;
    /// Restore the VM disk from a snapshot (VM must be stopped).
    async fn restore_snapshot(&self, vm: &VmConfig, snapshot_id: &str) -> anyhow::Result<()>;
    /// Clone `source` disk image into `target` (used by clone/template).
    async fn clone(&self, source: &VmConfig, target: &VmConfig) -> anyhow::Result<()>;
    /// Hot-resize a running VM; on stopped VMs only the config is updated.
    async fn resize(&self, vm: &VmConfig, cpus: u32, memory_mb: u32) -> anyhow::Result<()>;
    /// Hot-attach a data disk to a running VM.
    async fn attach_disk(&self, vm: &VmConfig, disk: &DiskConfig) -> anyhow::Result<()>;
    /// Hot-detach a data disk from a running VM.
    async fn detach_disk(&self, vm: &VmConfig, disk_id: &str) -> anyhow::Result<()>;
    /// Hot-attach a NIC to a running VM.
    async fn attach_nic(&self, vm: &VmConfig, tap: &str) -> anyhow::Result<()>;
    /// Hot-detach a NIC from a running VM.
    async fn detach_nic(&self, vm: &VmConfig, tap: &str) -> anyhow::Result<()>;
    /// Live-migrate a running VM to another cluster node (T1.4 HA). The VM must
    /// boot from shared storage (e.g. Ceph RBD) so the disk stays in place; only
    /// the memory/device state travels. On stopped VMs the backend is skipped and
    /// only placement moves.
    async fn migrate(&self, vm: &VmConfig, target_node: &str) -> anyhow::Result<()>;
    /// URL of the daemon-proxied console (VNC/vsock WebSocket endpoint).
    async fn console_url(&self, vm: &VmConfig) -> anyhow::Result<String>;
}

fn now_epoch() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

pub struct MockHypervisor {
    running: std::sync::Mutex<Vec<String>>,
    migrations: std::sync::Mutex<Vec<(String, String)>>,
}

impl MockHypervisor {
    pub fn new() -> Self {
        Self {
            running: std::sync::Mutex::new(Vec::new()),
            migrations: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Records of `(vm_id, target_node)` live migrations issued to this backend.
    pub fn migration_log(&self) -> Vec<(String, String)> {
        self.migrations.lock().unwrap().clone()
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

    async fn snapshot(&self, _vm: &VmConfig, name: &str) -> anyhow::Result<Snapshot> {
        Ok(Snapshot {
            id: format!("snap-{}", Uuid::new_v4()),
            name: name.to_string(),
            created_at: now_epoch(),
        })
    }

    async fn restore_snapshot(&self, _vm: &VmConfig, _snapshot_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn clone(&self, _source: &VmConfig, _target: &VmConfig) -> anyhow::Result<()> {
        Ok(())
    }

    async fn resize(&self, _vm: &VmConfig, _cpus: u32, _memory_mb: u32) -> anyhow::Result<()> {
        Ok(())
    }

    async fn attach_disk(&self, _vm: &VmConfig, _disk: &DiskConfig) -> anyhow::Result<()> {
        Ok(())
    }

    async fn detach_disk(&self, _vm: &VmConfig, _disk_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn attach_nic(&self, _vm: &VmConfig, _tap: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn detach_nic(&self, _vm: &VmConfig, _tap: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn migrate(&self, vm: &VmConfig, target_node: &str) -> anyhow::Result<()> {
        // The VM keeps running on the destination; only placement changes.
        if self.running.lock().unwrap().contains(&vm.id) {
            self.migrations
                .lock()
                .unwrap()
                .push((vm.id.clone(), target_node.to_string()));
        }
        Ok(())
    }

    async fn console_url(&self, vm: &VmConfig) -> anyhow::Result<String> {
        Ok(format!(
            "ws://127.0.0.1:8080/api/v1/vms/{}/console/ws",
            vm.id
        ))
    }
}

pub struct CloudHypervisor;

impl CloudHypervisor {
    pub fn new() -> Self {
        Self
    }

    fn socket(vm: &VmConfig) -> String {
        format!("/tmp/thiscloud-{}.sock", vm.id)
    }

    fn snapshot_dir(vm: &VmConfig, snapshot_id: &str) -> String {
        format!("/var/lib/thiscloud/snapshots/{}/{}", vm.id, snapshot_id)
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
        for disk in &vm.disks {
            cmd.arg("--disk").arg(format!("path={}", disk.path));
        }
        if vm.uefi {
            cmd.arg("--firmware").arg("/usr/share/OVMF/OVMF_CODE.fd");
        }
        if vm.tpm {
            cmd.arg("--vsock").arg("cid=3");
        }
        if let Some(user_data) = &vm.cloud_init {
            let path = format!("/var/lib/thiscloud/cloud-init/{}.yaml", vm.id);
            std::fs::write(&path, user_data)?;
            cmd.arg("--cloud-init").arg(format!("path={}", path));
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
            .arg(Self::socket(vm))
            .arg("shutdown")
            .status()
            .await?;
        Ok(())
    }

    async fn status(&self, vm: &VmConfig) -> anyhow::Result<VmStatus> {
        let output = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("info")
            .output()
            .await?;
        Ok(if output.status.success() {
            VmStatus::Running
        } else {
            VmStatus::Stopped
        })
    }

    async fn snapshot(&self, vm: &VmConfig, name: &str) -> anyhow::Result<Snapshot> {
        let id = format!("snap-{}", Uuid::new_v4());
        let status = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("snapshot")
            .arg("--path")
            .arg(Self::snapshot_dir(vm, &id))
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("cloud-hypervisor snapshot failed: {:?}", status);
        }
        Ok(Snapshot {
            id,
            name: name.to_string(),
            created_at: now_epoch(),
        })
    }

    async fn restore_snapshot(&self, vm: &VmConfig, snapshot_id: &str) -> anyhow::Result<()> {
        let status = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("restore")
            .arg("--path")
            .arg(Self::snapshot_dir(vm, snapshot_id))
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("cloud-hypervisor restore failed: {:?}", status);
        }
        Ok(())
    }

    async fn clone(&self, source: &VmConfig, target: &VmConfig) -> anyhow::Result<()> {
        let status = Command::new("qemu-img")
            .arg("create")
            .arg("-f")
            .arg("qcow2")
            .arg("-b")
            .arg(&source.disk_path)
            .arg("-F")
            .arg("qcow2")
            .arg(&target.disk_path)
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("qemu-img clone failed: {:?}", status);
        }
        Ok(())
    }

    async fn resize(&self, vm: &VmConfig, cpus: u32, memory_mb: u32) -> anyhow::Result<()> {
        let status = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("resize")
            .arg("--cpus")
            .arg(format!("boot={}", cpus))
            .arg("--memory")
            .arg(format!("size={}M", memory_mb))
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("cloud-hypervisor resize failed: {:?}", status);
        }
        Ok(())
    }

    async fn attach_disk(&self, vm: &VmConfig, disk: &DiskConfig) -> anyhow::Result<()> {
        let status = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("add-disk")
            .arg(format!("path={}", disk.path))
            .arg(disk.id.clone())
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("cloud-hypervisor add-disk failed: {:?}", status);
        }
        Ok(())
    }

    async fn detach_disk(&self, vm: &VmConfig, disk_id: &str) -> anyhow::Result<()> {
        let status = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("remove-disk")
            .arg(disk_id)
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("cloud-hypervisor remove-disk failed: {:?}", status);
        }
        Ok(())
    }

    async fn attach_nic(&self, vm: &VmConfig, tap: &str) -> anyhow::Result<()> {
        let status = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("add-net")
            .arg(format!("tap={}", tap))
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("cloud-hypervisor add-net failed: {:?}", status);
        }
        Ok(())
    }

    async fn detach_nic(&self, vm: &VmConfig, tap: &str) -> anyhow::Result<()> {
        let status = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("remove-net")
            .arg(tap)
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("cloud-hypervisor remove-net failed: {:?}", status);
        }
        Ok(())
    }

    async fn migrate(&self, vm: &VmConfig, target_node: &str) -> anyhow::Result<()> {
        if vm.status != VmStatus::Running {
            // Cold migration: no state to transfer; the orchestrator only moves
            // placement. Shared storage (Ceph RBD / Linstor) keeps the disk.
            return Ok(());
        }
        // Live migration: stream memory/device state to a ch endpoint on the
        // destination node. The destination daemon starts the receive side
        // (`cloud-hypervisor ... --migration src=tcp://<source>`) — in this
        // iteration the target is addressed by node, the real VhostUser/CH
        // broker wiring ships with the ISO agent.
        let dst = format!("tcp://{target_node}:8443");
        let status = Command::new("cloud-hypervisor")
            .arg("--api-socket")
            .arg(Self::socket(vm))
            .arg("migrate")
            .arg("--dst")
            .arg(dst)
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("cloud-hypervisor migrate failed: {:?}", status);
        }
        Ok(())
    }

    async fn console_url(&self, vm: &VmConfig) -> anyhow::Result<String> {
        Ok(format!(
            "ws://127.0.0.1:8080/api/v1/vms/{}/console/ws",
            vm.id
        ))
    }
}
