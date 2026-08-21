use crate::audit::model::{AuditEntry, AuditFilter};
use crate::core::EtcdClient;

/// Persistence abstraction for audit logs.
#[async_trait::async_trait]
pub trait AuditStore: Send + Sync {
    async fn log(&self, entry: AuditEntry);
    async fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry>;
}

/// In-memory bounded FIFO audit store (default; used in dev and tests).
pub struct MemoryAuditStore {
    entries: std::sync::Mutex<Vec<AuditEntry>>,
    max_entries: usize,
}

impl Default for MemoryAuditStore {
    fn default() -> Self {
        Self::with_capacity(10_000)
    }
}

impl MemoryAuditStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::with_capacity(max_entries.min(1024))),
            max_entries,
        }
    }
}

#[async_trait::async_trait]
impl AuditStore for MemoryAuditStore {
    async fn log(&self, entry: AuditEntry) {
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= self.max_entries {
            entries.remove(0);
        }
        entries.push(entry);
    }

    async fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        let entries = self.entries.lock().unwrap();
        let mut result: Vec<_> = entries
            .iter()
            .filter(|e| {
                filter
                    .tenant_id
                    .as_ref()
                    .map(|t| t == &e.tenant_id)
                    .unwrap_or(true)
            })
            .filter(|e| {
                filter
                    .user
                    .as_ref()
                    .map(|u| u == &e.user)
                    .unwrap_or(true)
            })
            .filter(|e| {
                filter
                    .action
                    .as_ref()
                    .map(|a| *a == e.action)
                    .unwrap_or(true)
            })
            .filter(|e| {
                filter
                    .resource
                    .as_ref()
                    .map(|r| r == &e.resource)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        // Most recent last.
        result.reverse();

        if let Some(limit) = filter.limit {
            result.truncate(limit);
        }
        result
    }
}

/// Append-only audit store persisted in etcd. Each entry is a new key under
/// `/thiscloud/audit/` and is never deleted, so the log is tamper-evident.
#[derive(Clone)]
pub struct EtcdAuditStore {
    client: EtcdClient,
}

impl EtcdAuditStore {
    pub fn new(client: EtcdClient) -> Self {
        Self { client }
    }

    fn key(entry: &AuditEntry) -> String {
        format!("/thiscloud/audit/{}-{}", entry.timestamp, entry.id)
    }
}

#[async_trait::async_trait]
impl AuditStore for EtcdAuditStore {
    async fn log(&self, entry: AuditEntry) {
        if let Ok(json) = serde_json::to_string(&entry) {
            if let Err(e) = self.client.put(&Self::key(&entry), &json).await {
                tracing::error!("failed to persist audit entry: {e}");
            }
        }
    }

    async fn query(&self, filter: &AuditFilter) -> Vec<AuditEntry> {
        match self.client.list_prefix("/thiscloud/audit/").await {
            Ok(entries) => {
                let mut all = Vec::new();
                for (_, json) in entries {
                    if let Ok(entry) = serde_json::from_str::<AuditEntry>(&json) {
                        all.push(entry);
                    }
                }
                // Oldest first in etcd ordering; apply the same filtering as memory.
                all.reverse();
                all.retain(|e| {
                    filter
                        .tenant_id
                        .as_ref()
                        .map(|t| t == &e.tenant_id)
                        .unwrap_or(true)
                        && filter.user.as_ref().map(|u| u == &e.user).unwrap_or(true)
                        && filter
                            .action
                            .as_ref()
                            .map(|a| *a == e.action)
                            .unwrap_or(true)
                        && filter
                            .resource
                            .as_ref()
                            .map(|r| r == &e.resource)
                            .unwrap_or(true)
                });
                all.reverse();
                if let Some(limit) = filter.limit {
                    all.truncate(limit);
                }
                all
            }
            Err(e) => {
                tracing::error!("failed to read audit log from etcd: {e}");
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::model::AuditAction;

    fn entry(resource: &str, action: AuditAction, tenant: &str) -> AuditEntry {
        AuditEntry {
            id: "1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            user: "u1".into(),
            role: "admin".into(),
            tenant_id: tenant.into(),
            action,
            resource: resource.into(),
            resource_id: "r1".into(),
            detail: String::new(),
        }
    }

    #[tokio::test]
    async fn log_and_query() {
        let store = MemoryAuditStore::new();
        store.log(entry("vm", AuditAction::Create, "t1")).await;
        store.log(entry("vm", AuditAction::Delete, "t1")).await;
        store
            .log(entry("network", AuditAction::Create, "t2"))
            .await;

        // Filter by tenant
        let result = store
            .query(&AuditFilter {
                tenant_id: Some("t1".into()),
                ..Default::default()
            })
            .await;
        assert_eq!(result.len(), 2);

        // Filter by action
        let result = store
            .query(&AuditFilter {
                action: Some(AuditAction::Delete),
                ..Default::default()
            })
            .await;
        assert_eq!(result.len(), 1);

        // Limit
        let result = store
            .query(&AuditFilter {
                limit: Some(1),
                ..Default::default()
            })
            .await;
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn bounded_fifo_evicts_oldest() {
        let store = MemoryAuditStore::with_capacity(3);
        for i in 0..5 {
            store
                .log(AuditEntry {
                    id: i.to_string(),
                    timestamp: String::new(),
                    user: String::new(),
                    role: String::new(),
                    tenant_id: String::new(),
                    action: AuditAction::Create,
                    resource: String::new(),
                    resource_id: String::new(),
                    detail: String::new(),
                })
                .await;
        }
        // Evicted first 2, kept last 3 (ids 2,3,4)
        let entries = store.entries.lock().unwrap();
        let ids: Vec<_> = entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["2", "3", "4"]);
    }
}
