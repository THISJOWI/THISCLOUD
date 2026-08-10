use thiscloudd::network::{LogicalNetwork, NetworkStatus};

#[test]
fn test_network_model_serde_roundtrip() {
    let net = LogicalNetwork::new(
        "net-1".to_string(),
        "web".to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    );
    let json = serde_json::to_string(&net).unwrap();
    let parsed: LogicalNetwork = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, net);
}

#[test]
fn test_network_model_defaults() {
    let net = LogicalNetwork::new(
        "net-1".to_string(),
        "web".to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    );
    assert_eq!(net.status, NetworkStatus::Created);
    assert_eq!(net.vlan, None);
    assert!(net.dns.is_empty());
}

#[test]
fn test_network_status_serde() {
    assert_eq!(
        serde_json::to_string(&NetworkStatus::Created).unwrap(),
        "\"created\""
    );
}
