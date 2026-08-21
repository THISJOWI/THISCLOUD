use serde::{Deserialize, Serialize};

/// Per-tenant resource limits. A value of 0 means "unlimited".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantQuota {
    pub tenant_id: String,
    #[serde(default)]
    pub max_cpus: u32,
    #[serde(default)]
    pub max_memory_mb: u32,
    #[serde(default)]
    pub max_vms: u32,
    #[serde(default)]
    pub max_storage_gb: u32,
    #[serde(default)]
    pub max_networks: u32,
}

impl TenantQuota {
    /// Default quota: everything unlimited.
    pub fn unlimited(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            max_cpus: 0,
            max_memory_mb: 0,
            max_vms: 0,
            max_storage_gb: 0,
            max_networks: 0,
        }
    }

    pub fn exceeds(&self, delta: &ResourceDelta) -> bool {
        (self.max_cpus != 0 && delta.cpus > self.max_cpus)
            || (self.max_memory_mb != 0 && delta.memory_mb > self.max_memory_mb)
            || (self.max_vms != 0 && delta.vms > self.max_vms)
            || (self.max_storage_gb != 0 && delta.storage_gb > self.max_storage_gb)
            || (self.max_networks != 0 && delta.networks > self.max_networks)
    }
}

/// Aggregated current usage for a tenant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceDelta {
    /// Total CPUs currently in use.
    pub cpus: u32,
    /// Total memory currently in use (MB).
    pub memory_mb: u32,
    /// Number of VMs currently running.
    pub vms: u32,
    /// Total storage currently allocated (GB).
    pub storage_gb: u32,
    /// Number of logical networks.
    pub networks: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_never_exceeds() {
        let q = TenantQuota::unlimited("t1");
        let d = ResourceDelta {
            cpus: 1000,
            memory_mb: 999999,
            vms: 500,
            storage_gb: 20000,
            networks: 100,
        };
        assert!(!q.exceeds(&d));
    }

    #[test]
    fn exceeds_limits() {
        let q = TenantQuota {
            tenant_id: "t1".into(),
            max_cpus: 4,
            max_memory_mb: 8192,
            max_vms: 2,
            max_storage_gb: 100,
            max_networks: 3,
        };
        assert!(q.exceeds(&ResourceDelta {
            cpus: 5,
            ..Default::default()
        }));
        assert!(q.exceeds(&ResourceDelta {
            memory_mb: 9000,
            ..Default::default()
        }));
        assert!(q.exceeds(&ResourceDelta {
            vms: 3,
            ..Default::default()
        }));
        assert!(!q.exceeds(&ResourceDelta {
            cpus: 3,
            memory_mb: 4000,
            ..Default::default()
        }));
    }
}