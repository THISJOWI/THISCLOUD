use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VmStatus {
    Running,
    Stopped,
    Error,
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
        }
    }
}
