use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditAction {
    Create,
    Read,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// JWT subject (username).
    pub user: String,
    /// JWT role.
    pub role: String,
    /// Tenant scope.
    pub tenant_id: String,
    /// Action performed.
    pub action: AuditAction,
    /// Resource type (e.g. "vm", "network", "storage_pool").
    pub resource: String,
    /// Resource identifier.
    pub resource_id: String,
    /// Free-form detail (e.g. "start", "stop").
    #[serde(default)]
    pub detail: String,
}

/// Filter for querying audit logs.
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub tenant_id: Option<String>,
    pub user: Option<String>,
    pub action: Option<AuditAction>,
    pub resource: Option<String>,
    pub limit: Option<usize>,
}