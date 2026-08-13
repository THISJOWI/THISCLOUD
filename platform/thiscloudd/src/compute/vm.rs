use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VmStatus {
    Running,
    Stopped,
    Error,
}

/// Additional data disk attached to a VM (the boot disk stays in `disk_path`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskConfig {
    #[serde(default)]
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub size_gb: u32,
}

/// Point-in-time snapshot of a VM disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

/// Console access info for a VM (VNC/vsock proxied by the daemon).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleInfo {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub cpus: u32,
    #[serde(default)]
    pub memory_mb: u32,
    #[serde(default)]
    pub disk_path: String,
    #[serde(default)]
    pub kernel: String,
    #[serde(default)]
    pub kernel_args: String,
    #[serde(default)]
    pub networks: Vec<String>,
    #[serde(default = "stopped")]
    pub status: VmStatus,
    #[serde(default)]
    pub tenant_id: String,
    /// Additional data disks (boot disk remains `disk_path`).
    #[serde(default)]
    pub disks: Vec<DiskConfig>,
    /// Point-in-time snapshots taken on this VM.
    #[serde(default)]
    pub snapshots: Vec<Snapshot>,
    /// Cloud-init user-data (ignition/cloud-config), passed at next boot.
    #[serde(default)]
    pub cloud_init: Option<String>,
    /// Boot with UEFI firmware (OVMF).
    #[serde(default)]
    pub uefi: bool,
    /// Attach a vTPM device (requires UEFI).
    #[serde(default)]
    pub tpm: bool,
    /// Marks the VM as a reusable template (never started directly).
    #[serde(default)]
    pub template: bool,
    /// Node to place the VM on. Empty means the best-fit scheduler picks it.
    #[serde(default)]
    pub node: String,
    /// Scheduler affinity labels (node must carry at least one).
    #[serde(default)]
    pub affinity: Vec<String>,
    /// Scheduler anti-affinity labels (node must not carry any).
    #[serde(default)]
    pub anti_affinity: Vec<String>,
    /// Image (registered in the image registry) used to boot this VM. Name or
    /// id. Derived disk_path when empty.
    #[serde(default)]
    pub image: String,
}

fn stopped() -> VmStatus {
    VmStatus::Stopped
}

impl VmConfig {
    pub fn new(
        id: String,
        name: String,
        cpus: u32,
        memory_mb: u32,
        disk_path: String,
        networks: Vec<String>,
    ) -> Self {
        Self {
            id,
            name,
            cpus,
            memory_mb,
            disk_path,
            kernel: String::new(),
            kernel_args: String::new(),
            networks,
            status: VmStatus::Stopped,
            tenant_id: String::new(),
            disks: Vec::new(),
            snapshots: Vec::new(),
            cloud_init: None,
            uefi: false,
            tpm: false,
            template: false,
            node: String::new(),
            affinity: Vec::new(),
            anti_affinity: Vec::new(),
            image: String::new(),
        }
    }
}
