use thiscloudd::compute::vm::{VmConfig, VmStatus};
use thiscloudd::compute::{ComputeModule, MemoryVmStore, MockHypervisor};
use thiscloudd::quota::model::TenantQuota;
use thiscloudd::quota::QuotaModule;
use std::sync::Arc;
use tokio::sync::Mutex;

fn sample_vm(name: &str) -> VmConfig {
    VmConfig::new(
        name.to_string(),
        name.to_string(),
        2,
        2048,
        format!("/var/lib/thiscloud/vms/{}.qcow2", name),
        vec!["br0".to_string()],
    )
}

#[tokio::test]
async fn test_compute_module_create_and_list() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module.create_vm("", sample_vm("web1")).await.unwrap();
    module.create_vm("", sample_vm("db1")).await.unwrap();

    let vms = module.list_vms("").await.unwrap();
    assert_eq!(vms.len(), 2);
}

#[tokio::test]
async fn test_compute_module_start_stop_updates_status() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module.create_vm("", sample_vm("web1")).await.unwrap();
    module.start_vm("", "web1").await.unwrap();

    let vms = module.list_vms("").await.unwrap();
    assert_eq!(vms[0].status, VmStatus::Running);

    module.stop_vm("", "web1").await.unwrap();
    let vms = module.list_vms("").await.unwrap();
    assert_eq!(vms[0].status, VmStatus::Stopped);
}

#[tokio::test]
async fn test_compute_module_delete() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module.create_vm("", sample_vm("web1")).await.unwrap();
    module.delete_vm("", "web1").await.unwrap();

    let vms = module.list_vms("").await.unwrap();
    assert!(vms.is_empty());
}

#[tokio::test]
async fn test_compute_module_start_missing_vm_errors() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    let result = module.start_vm("", "does-not-exist").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_compute_module_get_vm() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module.create_vm("", sample_vm("web1")).await.unwrap();
    let vm = module.get_vm("", "web1").await.unwrap();
    assert_eq!(vm.name, "web1");
    assert_eq!(vm.cpus, 2);
}

// --- T0.4: multitenancy tenant isolation ---

#[tokio::test]
async fn test_compute_module_tenant_isolation() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module
        .create_vm("tenant-a", sample_vm("web1"))
        .await
        .unwrap();

    // tenant-b cannot see tenant-a's VMs.
    assert!(module.list_vms("tenant-b").await.unwrap().is_empty());
    assert!(module.get_vm("tenant-b", "web1").await.is_err());

    // tenant-a sees its own VM.
    let vms_a = module.list_vms("tenant-a").await.unwrap();
    assert_eq!(vms_a.len(), 1);
    assert_eq!(vms_a[0].name, "web1");

    // Module stamped the tenant_id on the resource.
    assert_eq!(vms_a[0].tenant_id, "tenant-a");

    // Admin (empty tenant) sees all resources.
    assert_eq!(module.list_vms("").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_compute_module_multiple_tenants() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module
        .create_vm("tenant-a", sample_vm("a-vm"))
        .await
        .unwrap();
    module
        .create_vm("tenant-b", sample_vm("b-vm"))
        .await
        .unwrap();

    assert_eq!(module.list_vms("tenant-a").await.unwrap().len(), 1);
    assert_eq!(module.list_vms("tenant-b").await.unwrap().len(), 1);
    assert_eq!(module.list_vms("").await.unwrap().len(), 2);
}

// --- T0.5: quota enforcement ---

#[tokio::test]
async fn test_compute_module_quota_blocks_vm_create() {
    let quota_module = Arc::new(Mutex::new(QuotaModule::with_memory_store()));
    // max_vms = 1
    quota_module
        .lock()
        .await
        .set(TenantQuota {
            tenant_id: "tenant-a".into(),
            max_vms: 1,
            max_cpus: 8,
            max_memory_mb: 8192,
            ..TenantQuota::unlimited("tenant-a")
        })
        .await
        .unwrap();

    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store))
        .with_quota(quota_module);

    // First VM fits within the quota.
    module
        .create_vm("tenant-a", sample_vm("vm-1"))
        .await
        .unwrap();

    // Second VM exceeds max_vms=1 → error.
    let err = module.create_vm("tenant-a", sample_vm("vm-2")).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("exceeds quota"));
    assert_eq!(module.list_vms("tenant-a").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_compute_module_quota_blocks_cpu_memory() {
    let quota_module = Arc::new(Mutex::new(QuotaModule::with_memory_store()));
    quota_module
        .lock()
        .await
        .set(TenantQuota {
            tenant_id: "tenant-a".into(),
            max_vms: 10,
            max_cpus: 4,
            max_memory_mb: 4096,
            ..TenantQuota::unlimited("tenant-a")
        })
        .await
        .unwrap();

    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store))
        .with_quota(quota_module);

    // 2 cpus / 2048 MB → OK.
    module
        .create_vm("tenant-a", sample_vm("vm-1"))
        .await
        .unwrap();

    // Next VM would total 4 cpus / 4096 MB → exactly at limit → OK.
    module
        .create_vm("tenant-a", sample_vm("vm-2"))
        .await
        .unwrap();

    // Next VM would total 6 cpus / 6144 MB → exceeds max_cpus=4.
    let mut over = sample_vm("vm-3");
    over.cpus = 2;
    over.memory_mb = 2048;
    let err = module.create_vm("tenant-a", over).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("exceeds quota"));
}

#[tokio::test]
async fn test_compute_module_quota_unlimited_by_default() {
    let quota_module = Arc::new(Mutex::new(QuotaModule::with_memory_store()));
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store))
        .with_quota(quota_module);

    module
        .create_vm("tenant-a", sample_vm("vm-1"))
        .await
        .unwrap();
    module
        .create_vm("tenant-a", sample_vm("vm-2"))
        .await
        .unwrap();
    assert_eq!(module.list_vms("tenant-a").await.unwrap().len(), 2);
}
