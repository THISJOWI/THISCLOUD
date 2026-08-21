use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BackupConfig {
    /// Run a periodic snapshot on an interval.
    #[serde(default)]
    pub enabled: bool,
    /// Directory for snapshot files.
    #[serde(default = "default_dir")]
    pub dir: String,
    /// How many snapshots to keep (pruned oldest-first).
    #[serde(default = "default_retention")]
    pub retention: usize,
    /// Seconds between periodic snapshots.
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,
}

fn default_dir() -> String {
    "/var/lib/thiscloud/backup".to_string()
}

fn default_retention() -> usize {
    7
}

fn default_interval_secs() -> u64 {
    3600
}