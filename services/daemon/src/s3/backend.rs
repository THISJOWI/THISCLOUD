use crate::s3::S3Bucket;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Mutex;

#[async_trait]
pub trait S3Backend: Send + Sync {
    /// Provision the RadosGW user for a tenant (idempotent in practice).
    async fn create_user(&self, tenant_id: &str) -> anyhow::Result<()>;
    async fn create_bucket(&self, bucket: &S3Bucket) -> anyhow::Result<()>;
    async fn delete_bucket(&self, bucket: &S3Bucket) -> anyhow::Result<()>;
    async fn exists_bucket(&self, name: &str) -> anyhow::Result<bool>;
}

/// In-memory backend used by tests and macOS dev, mirroring MockStorageBackend.
#[derive(Default)]
pub struct MockS3Backend {
    users: Mutex<HashSet<String>>,
    buckets: Mutex<HashSet<String>>,
}

impl MockS3Backend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl S3Backend for MockS3Backend {
    async fn create_user(&self, tenant_id: &str) -> anyhow::Result<()> {
        self.users.lock().unwrap().insert(tenant_id.to_string());
        Ok(())
    }

    async fn create_bucket(&self, bucket: &S3Bucket) -> anyhow::Result<()> {
        self.buckets.lock().unwrap().insert(bucket.name.clone());
        Ok(())
    }

    async fn delete_bucket(&self, bucket: &S3Bucket) -> anyhow::Result<()> {
        self.buckets.lock().unwrap().remove(&bucket.name);
        Ok(())
    }

    async fn exists_bucket(&self, name: &str) -> anyhow::Result<bool> {
        Ok(self.buckets.lock().unwrap().contains(name))
    }
}

/// Real backend that shells out to `radosgw-admin` (Ceph object gateway).
/// Only runs inside the ISO; on dev the mock backend is used.
pub struct RadosgwBackend {
    gateway_host: String,
}

impl RadosgwBackend {
    pub fn new(gateway_host: String) -> Self {
        Self { gateway_host }
    }

    pub fn create_user_command(&self, tenant_id: &str) -> Vec<String> {
        vec![
            "radosgw-admin".to_string(),
            "user".to_string(),
            "create".to_string(),
            format!("--uid=thiscloud-{tenant_id}"),
            "--display-name=thiscloud".to_string(),
        ]
    }

    pub fn create_subuser_command(&self, tenant_id: &str) -> Vec<String> {
        vec![
            "radosgw-admin".to_string(),
            "subuser".to_string(),
            "create".to_string(),
            format!("--uid=thiscloud-{tenant_id}"),
            format!("--subuser=thiscloud-{tenant_id}:swift"),
            "--access=full".to_string(),
        ]
    }

    pub fn create_bucket_command(&self, bucket: &S3Bucket) -> Vec<String> {
        vec![
            "radosgw-admin".to_string(),
            "bucket".to_string(),
            "create".to_string(),
            format!("--bucket={}", bucket.name),
            format!("--uid=thiscloud-{}", bucket.tenant_id),
        ]
    }

    pub fn delete_bucket_command(&self, bucket: &S3Bucket) -> Vec<String> {
        vec![
            "radosgw-admin".to_string(),
            "bucket".to_string(),
            "rm".to_string(),
            format!("--bucket={}", bucket.name),
            "--purge-objects".to_string(),
        ]
    }

    pub fn bucket_stats_command(&self, name: &str) -> Vec<String> {
        vec![
            "radosgw-admin".to_string(),
            "bucket".to_string(),
            "stats".to_string(),
            format!("--bucket={name}"),
        ]
    }

    async fn run(&self, cmd: &[String]) -> anyhow::Result<()> {
        tracing::debug!("radosgw-admin (gateway {}) {:?}", self.gateway_host, cmd);
        let status = tokio::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("radosgw-admin command failed: {:?}", status)
        }
    }
}

#[async_trait]
impl S3Backend for RadosgwBackend {
    async fn create_user(&self, tenant_id: &str) -> anyhow::Result<()> {
        self.run(&self.create_user_command(tenant_id)).await?;
        self.run(&self.create_subuser_command(tenant_id)).await
    }

    async fn create_bucket(&self, bucket: &S3Bucket) -> anyhow::Result<()> {
        self.run(&self.create_bucket_command(bucket)).await
    }

    async fn delete_bucket(&self, bucket: &S3Bucket) -> anyhow::Result<()> {
        self.run(&self.delete_bucket_command(bucket)).await
    }

    async fn exists_bucket(&self, name: &str) -> anyhow::Result<bool> {
        let output = tokio::process::Command::new("radosgw-admin")
            .args(["bucket", "stats", &format!("--bucket={name}")])
            .output()
            .await?;
        // `bucket stats` exits non-zero when the bucket does not exist.
        Ok(output.status.success())
    }
}