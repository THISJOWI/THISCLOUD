use thiscloudd::compute::vm::{VmConfig, VmStatus};
use thiscloudd::compute::{ComputeModule, MemoryVmStore, MockHypervisor};

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

    module.create_vm(sample_vm("web1")).await.unwrap();
    module.create_vm(sample_vm("db1")).await.unwrap();

    let vms = module.list_vms().await.unwrap();
    assert_eq!(vms.len(), 2);
}

#[tokio::test]
async fn test_compute_module_start_stop_updates_status() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module.create_vm(sample_vm("web1")).await.unwrap();
    module.start_vm("web1").await.unwrap();

    let vms = module.list_vms().await.unwrap();
    assert_eq!(vms[0].status, VmStatus::Running);

    module.stop_vm("web1").await.unwrap();
    let vms = module.list_vms().await.unwrap();
    assert_eq!(vms[0].status, VmStatus::Stopped);
}

#[tokio::test]
async fn test_compute_module_delete() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module.create_vm(sample_vm("web1")).await.unwrap();
    module.delete_vm("web1").await.unwrap();

    let vms = module.list_vms().await.unwrap();
    assert!(vms.is_empty());
}

#[tokio::test]
async fn test_compute_module_start_missing_vm_errors() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    let result = module.start_vm("does-not-exist").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_compute_module_get_vm() {
    let store = MemoryVmStore::default();
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(store));

    module.create_vm(sample_vm("web1")).await.unwrap();
    let vm = module.get_vm("web1").await.unwrap();
    assert_eq!(vm.name, "web1");
    assert_eq!(vm.cpus, 2);
}
