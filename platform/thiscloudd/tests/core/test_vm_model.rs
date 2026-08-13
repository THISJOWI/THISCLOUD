use thiscloudd::compute::vm::{VmConfig, VmStatus};

#[test]
fn test_vm_config_roundtrip() {
    let config = VmConfig {
        id: "vm-1".to_string(),
        name: "web1".to_string(),
        cpus: 2,
        memory_mb: 2048,
        disk_path: "/var/lib/thiscloud/vms/web1.qcow2".to_string(),
        kernel: "/boot/vmlinuz".to_string(),
        kernel_args: "console=ttyS0".to_string(),
        networks: vec!["br0".to_string()],
        status: VmStatus::Stopped,
        tenant_id: String::new(),
        disks: Vec::new(),
        snapshots: Vec::new(),
        cloud_init: None,
        uefi: false,
        tpm: false,
        template: false,
        node: String::new(),
        affinity: Vec::new(),
        anti_affinity: Vec::new(),
    };

    let json = serde_json::to_string(&config).unwrap();
    let parsed: VmConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, "vm-1");
    assert_eq!(parsed.name, "web1");
    assert_eq!(parsed.cpus, 2);
    assert_eq!(parsed.memory_mb, 2048);
    assert_eq!(parsed.networks, vec!["br0".to_string()]);
    assert_eq!(parsed.status, VmStatus::Stopped);
}

#[test]
fn test_vm_status_serialization() {
    assert_eq!(
        serde_json::to_string(&VmStatus::Running).unwrap(),
        "\"running\""
    );
    assert_eq!(
        serde_json::to_string(&VmStatus::Stopped).unwrap(),
        "\"stopped\""
    );
    assert_eq!(
        serde_json::to_string(&VmStatus::Error).unwrap(),
        "\"error\""
    );
}

#[test]
fn test_vm_config_new() {
    let config = VmConfig::new(
        "vm-2".to_string(),
        "db1".to_string(),
        4,
        4096,
        "/var/lib/thiscloud/vms/db1.qcow2".to_string(),
        vec!["br0".to_string()],
    );

    assert_eq!(config.id, "vm-2");
    assert_eq!(config.name, "db1");
    assert_eq!(config.cpus, 4);
    assert_eq!(config.memory_mb, 4096);
    assert_eq!(config.status, VmStatus::Stopped);
}
