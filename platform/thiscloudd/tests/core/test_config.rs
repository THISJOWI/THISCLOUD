use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_cluster_config_from_file() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");

    let config_content = r#"
name = "test-cluster"

[[nodes]]
ip = "10.0.0.10"
role = "master"

[[nodes]]
ip = "10.0.0.11"
role = "worker"
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = thiscloudd::config::ClusterConfig::load(&config_path).unwrap();

    assert_eq!(config.name, "test-cluster");
    assert_eq!(config.nodes.len(), 2);
    assert_eq!(config.nodes[0].ip, "10.0.0.10");
    assert_eq!(config.nodes[0].role, "master");
    assert_eq!(config.nodes[1].ip, "10.0.0.11");
    assert_eq!(config.nodes[1].role, "worker");
}

#[test]
fn test_load_storage_config_from_file() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");

    let config_content = r#"
[[pools]]
name = "ssd-pool"
type = "lvm-thin"
devices = ["/dev/nvme0n1"]
replication = 2
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = thiscloudd::config::StorageConfig::load(&config_path).unwrap();

    assert_eq!(config.pools.len(), 1);
    assert_eq!(config.pools[0].name, "ssd-pool");
    assert_eq!(config.pools[0].pool_type, "lvm-thin");
    assert_eq!(config.pools[0].replication, 2);
}

#[test]
fn test_network_config_defaults() {
    let config = thiscloudd::config::NetworkConfig::default();
    assert_eq!(config.management_vlan, 100);
    assert_eq!(config.overlay_type, "geneve");
}

#[test]
fn test_etcd_config_defaults() {
    let config = thiscloudd::config::EtcdConfig::default();
    assert!(config.embedded);
    assert_eq!(config.port, 2379);
    assert_eq!(config.peer_port, 2380);
    assert_eq!(config.data_dir, "/var/lib/thiscloud/etcd");
}

#[test]
fn test_full_config_load() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");

    let config_content = r#"
[cluster]
name = "thiscloud-prod"

[cluster.etcd]
embedded = true
port = 2379
peer_port = 2380
data_dir = "/var/lib/thiscloud/etcd"
quota_backend = "8GB"

[[cluster.nodes]]
ip = "10.0.0.10"
role = "master"

[storage]

[[storage.pools]]
name = "ssd-pool"
type = "lvm-thin"
devices = ["/dev/nvme0n1"]
replication = 2

[network]
management_vlan = 100
overlay_type = "geneve"
"#;

    fs::write(&config_path, config_content).unwrap();

    let config = thiscloudd::config::ThisCloudConfig::load(&config_path).unwrap();

    assert_eq!(config.cluster.name, "thiscloud-prod");
    assert_eq!(config.cluster.nodes.len(), 1);
    assert!(config.cluster.etcd.embedded);
    assert_eq!(config.cluster.etcd.quota_backend, "8GB");
    assert_eq!(config.storage.pools.len(), 1);
    assert_eq!(config.storage.pools[0].name, "ssd-pool");
    assert_eq!(config.network.management_vlan, 100);
    assert_eq!(config.network.overlay_type, "geneve");
}

#[test]
fn test_full_config_defaults() {
    let config = thiscloudd::config::ThisCloudConfig::default();
    assert_eq!(config.cluster.name, "thiscloud");
    assert!(config.cluster.nodes.is_empty());
    assert!(config.storage.pools.is_empty());
    assert_eq!(config.network.management_vlan, 100);
    assert_eq!(config.network.overlay_type, "geneve");
}
