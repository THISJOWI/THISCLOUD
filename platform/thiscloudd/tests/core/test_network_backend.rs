use thiscloudd::network::{LogicalNetwork, MockNetworkBackend, NetworkBackend, OvnNetworkBackend};

fn sample_net(id: &str) -> LogicalNetwork {
    LogicalNetwork::new(
        id.to_string(),
        id.to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    )
}

#[tokio::test]
async fn test_mock_backend_create_and_exists() {
    let backend = MockNetworkBackend::new();
    let net = sample_net("net-1");
    backend.create(&net).await.unwrap();
    assert!(backend.exists("net-1").await.unwrap());
    assert!(!backend.exists("nope").await.unwrap());
}

#[tokio::test]
async fn test_mock_backend_delete() {
    let backend = MockNetworkBackend::new();
    let net = sample_net("net-1");
    backend.create(&net).await.unwrap();
    backend.delete(&net).await.unwrap();
    assert!(!backend.exists("net-1").await.unwrap());
}

#[test]
fn test_ovn_backend_create_command() {
    let backend = OvnNetworkBackend::new();
    let net = sample_net("net-1");
    let cmd = backend.create_command(&net);
    assert_eq!(cmd, vec!["ovn-nbctl", "ls-add", "net-1"]);
}

#[test]
fn test_ovn_backend_delete_command() {
    let backend = OvnNetworkBackend::new();
    let net = sample_net("net-1");
    let cmd = backend.delete_command(&net);
    assert_eq!(cmd, vec!["ovn-nbctl", "ls-del", "net-1"]);
}

#[test]
fn test_ovn_backend_set_subnet_command() {
    let backend = OvnNetworkBackend::new();
    let net = sample_net("net-1");
    let cmd = backend.set_subnet_command(&net);
    assert_eq!(
        cmd,
        vec![
            "ovn-nbctl",
            "set",
            "Logical_Switch",
            "net-1",
            "other_config:subnet=10.0.0.0/24"
        ]
    );
}
