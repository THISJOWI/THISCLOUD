use thiscloudd::compute::vm::{VmConfig, VmStatus};
use thiscloudd::compute::{HypervisorBackend, MockHypervisor};

fn sample_vm(id: &str) -> VmConfig {
    VmConfig::new(
        id.to_string(),
        format!("vm-{}", id),
        2,
        2048,
        format!("/var/lib/thiscloud/vms/{}.qcow2", id),
        vec!["br0".to_string()],
    )
}

#[tokio::test]
async fn test_mock_spawn_and_status() {
    let backend = MockHypervisor::new();
    let vm = sample_vm("m1");

    assert_eq!(backend.status(&vm).await.unwrap(), VmStatus::Stopped);

    backend.spawn(&vm).await.unwrap();
    assert_eq!(backend.status(&vm).await.unwrap(), VmStatus::Running);

    backend.stop(&vm).await.unwrap();
    assert_eq!(backend.status(&vm).await.unwrap(), VmStatus::Stopped);
}

#[tokio::test]
async fn test_mock_multiple_vms_independent() {
    let backend = MockHypervisor::new();
    let vm_a = sample_vm("ma");
    let vm_b = sample_vm("mb");

    backend.spawn(&vm_a).await.unwrap();

    assert_eq!(backend.status(&vm_a).await.unwrap(), VmStatus::Running);
    assert_eq!(backend.status(&vm_b).await.unwrap(), VmStatus::Stopped);
}

#[tokio::test]
async fn test_mock_stop_nonrunning_is_ok() {
    let backend = MockHypervisor::new();
    let vm = sample_vm("m2");

    backend.stop(&vm).await.unwrap();
    assert_eq!(backend.status(&vm).await.unwrap(), VmStatus::Stopped);
}
