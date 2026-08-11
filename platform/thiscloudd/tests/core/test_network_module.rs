use thiscloudd::core::Module;
use thiscloudd::network::{LogicalNetwork, MemoryNetworkStore, MockNetworkBackend, NetworkModule};

fn sample_net(id: &str) -> LogicalNetwork {
    LogicalNetwork::new(
        id.to_string(),
        id.to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    )
}

fn module() -> NetworkModule {
    NetworkModule::new(
        Box::new(MockNetworkBackend::new()),
        Box::new(MemoryNetworkStore::default()),
    )
}

#[tokio::test]
async fn test_network_module_create_and_list() {
    let mut m = module();
    m.create_network("", &mut sample_net("web")).await.unwrap();
    m.create_network("", &mut sample_net("db")).await.unwrap();
    assert_eq!(m.list_networks("").await.unwrap().len(), 2);
}

#[tokio::test]
async fn test_network_module_get() {
    let mut m = module();
    m.create_network("", &mut sample_net("web")).await.unwrap();
    let net = m.get_network("", "web").await.unwrap();
    assert_eq!(net.name, "web");
    assert_eq!(net.cidr, "10.0.0.0/24");
}

#[tokio::test]
async fn test_network_module_get_missing_errors() {
    let m = module();
    assert!(m.get_network("", "nope").await.is_err());
}

#[tokio::test]
async fn test_network_module_duplicate_name_errors() {
    let mut m = module();
    m.create_network("", &mut sample_net("web")).await.unwrap();
    let mut dup = sample_net("web");
    let err = m.create_network("", &mut dup).await.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_network_module_delete() {
    let mut m = module();
    m.create_network("", &mut sample_net("web")).await.unwrap();
    m.delete_network("", "web").await.unwrap();
    assert!(m.list_networks("").await.unwrap().is_empty());
}

#[tokio::test]
async fn test_network_module_name() {
    assert_eq!(module().name(), "network");
}
