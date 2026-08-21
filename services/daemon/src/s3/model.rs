use serde::{Deserialize, Serialize};

/// An S3 bucket owned by a tenant, backed by Ceph RadosGW.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct S3Bucket {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tenant_id: String,
    #[serde(default)]
    pub created_at: String,
}

impl S3Bucket {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            tenant_id: String::new(),
            created_at: String::new(),
        }
    }
}

/// S3 credentials (access key / secret key) issued for a tenant's RadosGW user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct S3AccessKey {
    pub access_key: String,
    pub secret_key: String,
    /// RadosGW user the key belongs to (e.g. `thiscloud-<tenant>`).
    pub user: String,
    #[serde(default)]
    pub tenant_id: String,
}

impl S3AccessKey {
    pub fn new(access_key: String, secret_key: String, user: String) -> Self {
        Self {
            access_key,
            secret_key,
            user,
            tenant_id: String::new(),
        }
    }
}