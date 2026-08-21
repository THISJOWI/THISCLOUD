use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    Master,
    Worker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeState {
    Online,
    Offline,
    Draining,
}

/// A cluster node registered with the master daemon.
///
/// Capacity values of `0` mean "unknown/unlimited" so a self-registered master
/// with no hard limits reported stays schedulable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default = "default_role")]
    pub role: NodeRole,
    /// ip:port of the node agent (worker daemon).
    pub address: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub cpus_total: u32,
    #[serde(default)]
    pub cpus_used: u32,
    #[serde(default)]
    pub memory_total_mb: u32,
    #[serde(default)]
    pub memory_used_mb: u32,
    #[serde(default)]
    pub vms: u32,
    #[serde(default = "default_state")]
    pub state: NodeState,
    /// UNIX epoch seconds of the last heartbeat.
    #[serde(default)]
    pub last_seen_secs: u64,
    /// Seconds without a heartbeat before the node is considered offline.
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,
    /// Scheduler labels used for affinity/anti-affinity matching.
    #[serde(default)]
    pub labels: Vec<String>,
}

fn default_role() -> NodeRole {
    NodeRole::Worker
}

fn default_state() -> NodeState {
    NodeState::Online
}

fn default_ttl() -> u64 {
    30
}

impl Node {
    pub fn is_capable(&self, cpus: u32, memory_mb: u32) -> bool {
        let cpu_ok = self.cpus_total == 0 || self.cpus_used + cpus <= self.cpus_total;
        let mem_ok = self.memory_total_mb == 0 || self.memory_used_mb + memory_mb <= self.memory_total_mb;
        cpu_ok && mem_ok
    }

    pub fn load_ratio(&self) -> f64 {
        if self.cpus_total == 0 {
            return 0.0;
        }
        self.cpus_used as f64 / self.cpus_total as f64
    }
}

/// Payload reported by a node agent on heartbeat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHeartbeat {
    pub cpus_used: u32,
    pub memory_used_mb: u32,
    pub vms: u32,
}
