use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Operator,
    TenantAdmin,
    TenantUser,
    Auditor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user or service account id)
    pub sub: String,
    /// Tenant scope (empty for admin)
    pub tenant_id: String,
    /// Role
    pub role: Role,
    /// Expiration (unix timestamp)
    pub exp: usize,
    /// Issued at (unix timestamp)
    pub iat: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    /// Hashed key (never stored raw)
    pub key_hash: String,
    pub name: String,
    pub tenant_id: String,
    pub role: Role,
    pub created_at: String,
}
