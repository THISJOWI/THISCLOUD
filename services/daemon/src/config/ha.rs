use serde::Deserialize;

/// High-availability tuning (T1.4): automatic failover of VMs off dead nodes.
#[derive(Debug, Clone, Deserialize)]
pub struct HaConfig {
    /// Master switch for HA failover.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Minimum number of online nodes required to authorize a failover
    /// (`online < quorum` blocks relocation, preventing split-brain). The
    /// effective quorum is `max(quorum, registered/2 + 1)`.
    #[serde(default = "default_quorum")]
    pub quorum: u32,
    /// Seconds between automatic HA scans.
    #[serde(default = "default_scan_interval_secs")]
    pub scan_interval_secs: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_quorum() -> u32 {
    2
}

fn default_scan_interval_secs() -> u64 {
    10
}

impl Default for HaConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            quorum: default_quorum(),
            scan_interval_secs: default_scan_interval_secs(),
        }
    }
}