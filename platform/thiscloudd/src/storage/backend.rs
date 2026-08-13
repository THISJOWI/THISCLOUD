use crate::storage::StoragePool;
use std::collections::HashSet;
use std::sync::Mutex;

#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync {
    async fn create(&self, pool: &StoragePool) -> anyhow::Result<()>;
    async fn delete(&self, pool: &StoragePool) -> anyhow::Result<()>;
    async fn exists(&self, name: &str) -> anyhow::Result<bool>;
}

#[derive(Default)]
pub struct MockStorageBackend {
    pools: Mutex<HashSet<String>>,
}

impl MockStorageBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl StorageBackend for MockStorageBackend {
    async fn create(&self, pool: &StoragePool) -> anyhow::Result<()> {
        self.pools.lock().unwrap().insert(pool.name.clone());
        Ok(())
    }

    async fn delete(&self, pool: &StoragePool) -> anyhow::Result<()> {
        self.pools.lock().unwrap().remove(&pool.name);
        Ok(())
    }

    async fn exists(&self, name: &str) -> anyhow::Result<bool> {
        Ok(self.pools.lock().unwrap().contains(name))
    }
}

pub struct LinstorBackend;

impl Default for LinstorBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LinstorBackend {
    pub fn new() -> Self {
        Self
    }

    pub fn create_command(&self, pool: &StoragePool) -> Vec<String> {
        vec![
            "linstor".to_string(),
            "storage-pool".to_string(),
            "create".to_string(),
            "file".to_string(),
            pool.name.clone(),
            format!("--replicas={}", pool.replication),
        ]
    }

    pub fn delete_command(&self, pool: &StoragePool) -> Vec<String> {
        vec![
            "linstor".to_string(),
            "storage-pool".to_string(),
            "delete".to_string(),
            pool.name.clone(),
        ]
    }
}

#[async_trait::async_trait]
impl StorageBackend for LinstorBackend {
    async fn create(&self, pool: &StoragePool) -> anyhow::Result<()> {
        self.run(&self.create_command(pool)).await
    }

    async fn delete(&self, pool: &StoragePool) -> anyhow::Result<()> {
        self.run(&self.delete_command(pool)).await
    }

    async fn exists(&self, name: &str) -> anyhow::Result<bool> {
        let output = tokio::process::Command::new("linstor")
            .args(["storage-pool", "list"])
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().any(|l| l.contains(name)))
    }
}

impl LinstorBackend {
    async fn run(&self, cmd: &[String]) -> anyhow::Result<()> {
        let status = tokio::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("linstor command failed: {:?}", status)
        }
    }
}

const CEPH_DEFAULT_PG_NUM: u32 = 32;

pub struct CephBackend {
    pg_num: u32,
}

impl Default for CephBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CephBackend {
    pub fn new() -> Self {
        Self {
            pg_num: CEPH_DEFAULT_PG_NUM,
        }
    }

    pub fn with_pg_num(pg_num: u32) -> Self {
        Self { pg_num }
    }

    pub fn create_command(&self, pool: &StoragePool) -> Vec<String> {
        vec![
            "ceph".to_string(),
            "osd".to_string(),
            "pool".to_string(),
            "create".to_string(),
            pool.name.clone(),
            self.pg_num.to_string(),
            "replicated".to_string(),
        ]
    }

    pub fn pool_set_size_command(&self, pool: &StoragePool) -> Vec<String> {
        vec![
            "ceph".to_string(),
            "osd".to_string(),
            "pool".to_string(),
            "set".to_string(),
            pool.name.clone(),
            "size".to_string(),
            pool.replication.to_string(),
        ]
    }

    pub fn application_enable_command(&self, pool: &StoragePool) -> Vec<String> {
        vec![
            "ceph".to_string(),
            "osd".to_string(),
            "pool".to_string(),
            "application".to_string(),
            "enable".to_string(),
            pool.name.clone(),
            "rbd".to_string(),
        ]
    }

    pub fn delete_command(&self, pool: &StoragePool) -> Vec<String> {
        vec![
            "ceph".to_string(),
            "osd".to_string(),
            "pool".to_string(),
            "rm".to_string(),
            pool.name.clone(),
            pool.name.clone(),
            "--yes-i-really-really-mean-it".to_string(),
        ]
    }
}

#[async_trait::async_trait]
impl StorageBackend for CephBackend {
    async fn create(&self, pool: &StoragePool) -> anyhow::Result<()> {
        self.run(&self.create_command(pool)).await?;
        self.run(&self.pool_set_size_command(pool)).await?;
        self.run(&self.application_enable_command(pool)).await
    }

    async fn delete(&self, pool: &StoragePool) -> anyhow::Result<()> {
        self.run(&self.delete_command(pool)).await
    }

    async fn exists(&self, name: &str) -> anyhow::Result<bool> {
        let output = tokio::process::Command::new("ceph")
            .args(["osd", "pool", "ls"])
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().any(|l| l.trim() == name))
    }
}

impl CephBackend {
    async fn run(&self, cmd: &[String]) -> anyhow::Result<()> {
        let status = tokio::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("ceph command failed: {:?}", status)
        }
    }
}
