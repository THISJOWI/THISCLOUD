use crate::core::EtcdClient;
use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod http;

/// A snapshot of the full etcd state. Backups are plain JSON files on local
/// disk (S3 offsite is T3.2), one per point in time, pruned by retention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub format: String,
    pub created_at: u64,
    pub entries: Vec<(String, String)>,
}

impl Snapshot {
    pub const FORMAT: &'static str = "thiscloud-backup-v1";
}

pub struct BackupService {
    etcd: Option<EtcdClient>,
    dir: PathBuf,
    retention: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotInfo {
    pub name: String,
    pub created_at: u64,
    pub size_bytes: u64,
    pub entries: usize,
}

impl BackupService {
    pub fn new(etcd: Option<EtcdClient>, dir: impl Into<PathBuf>, retention: usize) -> Self {
        Self {
            etcd,
            dir: dir.into(),
            retention: retention.max(1),
        }
    }

    /// Snapshot the current etcd state to a JSON file and prune old snapshots.
    pub async fn create_snapshot(&self) -> anyhow::Result<SnapshotInfo> {
        let client = self
            .etcd
            .as_ref()
            .ok_or_else(|| anyhow!("backup requires a persistent store (etcd); the daemon is running on in-memory stores"))?;

        let entries = client.dump().await.context("dump etcd state")?;
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let snapshot = Snapshot {
            format: Snapshot::FORMAT.to_string(),
            created_at,
            entries,
        };

        std::fs::create_dir_all(&self.dir)
            .with_context(|| format!("create backup dir {}", self.dir.display()))?;
        let name = format!("thiscloud-{}.json", created_at);
        let path = self.dir.join(&name);
        let data = serde_json::to_vec_pretty(&snapshot).context("serialize snapshot")?;
        std::fs::write(&path, &data)
            .with_context(|| format!("write snapshot {}", path.display()))?;

        let info = SnapshotInfo {
            name,
            created_at,
            size_bytes: data.len() as u64,
            entries: snapshot.entries.len(),
        };

        self.prune().await?;
        Ok(info)
    }

    /// Restore a snapshot by wiping the current state and replaying it.
    pub async fn restore_snapshot(&self, name: &str) -> anyhow::Result<SnapshotInfo> {
        let client = self
            .etcd
            .as_ref()
            .ok_or_else(|| anyhow!("restore requires a persistent store (etcd); the daemon is running on in-memory stores"))?;

        let path = self.snapshot_path(name)?;
        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("read snapshot {}", path.display()))?;
        let snapshot: Snapshot = serde_json::from_str(&data).context("parse snapshot")?;
        if snapshot.format != Snapshot::FORMAT {
            bail!("unsupported snapshot format: {}", snapshot.format);
        }

        client.wipe().await.context("wipe current etcd state")?;
        client
            .write_all(&snapshot.entries)
            .await
            .context("replay snapshot entries")?;

        Ok(SnapshotInfo {
            name: name.to_string(),
            created_at: snapshot.created_at,
            size_bytes: data.len() as u64,
            entries: snapshot.entries.len(),
        })
    }

    /// List available snapshots, newest first.
    pub fn list_snapshots(&self) -> anyhow::Result<Vec<SnapshotInfo>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.dir).context("read backup dir")? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("thiscloud-") || !name.ends_with(".json") {
                continue;
            }
            let size_bytes = entry.metadata()?.len();
            let created_at = name
                .trim_start_matches("thiscloud-")
                .trim_end_matches(".json")
                .parse::<u64>()
                .unwrap_or(0);
            out.push(SnapshotInfo {
                entries: 0,
                name,
                created_at,
                size_bytes,
            });
        }
        out.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(out)
    }

    /// Prune snapshots beyond the retention window. Returns removed names.
    pub async fn prune(&self) -> anyhow::Result<Vec<String>> {
        let snapshots = self.list_snapshots()?;
        let mut removed = Vec::new();
        for snap in snapshots.iter().skip(self.retention) {
            let path = self.dir.join(&snap.name);
            std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
            removed.push(snap.name.clone());
        }
        Ok(removed)
    }

    /// Resolve a snapshot name to a path, rejecting path traversal.
    fn snapshot_path(&self, name: &str) -> anyhow::Result<PathBuf> {
        let path = self.dir.join(name);
        if !name.starts_with("thiscloud-")
            || !name.ends_with(".json")
            || name.contains('/')
            || name.contains("..")
        {
            bail!("invalid snapshot name: {}", name);
        }
        if !path.exists() {
            bail!("snapshot not found: {}", name);
        }
        Ok(path)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}