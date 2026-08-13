use thiscloudd::core::Module;
use thiscloudd::network::{LogicalNetwork, MemoryNetworkStore, MockNetworkBackend, NetworkModule};
use thiscloudd::quota::model::TenantQuota;
use thiscloudd::quota::QuotaModule;
use std::sync::Arc;
use tokio::sync::Mutex;

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

// --- T0.5: quota enforcement ---

#[tokio::test]
async fn test_network_module_quota_blocks_create() {
    let quota_module = Arc::new(Mutex::new(QuotaModule::with_memory_store()));
    quota_module
        .lock()
        .await
        .set(TenantQuota {
            tenant_id: "tenant-a".into(),
            max_networks: 1,
            ..TenantQuota::unlimited("tenant-a")
        })
        .await
        .unwrap();

    let mut m = NetworkModule::new(
        Box::new(MockNetworkBackend::new()),
        Box::new(MemoryNetworkStore::default()),
    )
    .with_quota(quota_module);

    m.create_network("tenant-a", &mut sample_net("web"))
        .await
        .unwrap();

    let err = m.create_network("tenant-a", &mut sample_net("db")).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("exceeds quota"));
    assert_eq!(m.list_networks("tenant-a").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_network_module_quota_scoped_per_tenant() {
    let quota_module = Arc::new(Mutex::new(QuotaModule::with_memory_store()));
    quota_module
        .lock()
        .await
        .set(TenantQuota {
            tenant_id: "tenant-a".into(),
            max_networks: 1,
            ..TenantQuota::unlimited("tenant-a")
        })
        .await
        .unwrap();

    let mut m = NetworkModule::new(
        Box::new(MockNetworkBackend::new()),
        Box::new(MemoryNetworkStore::default()),
    )
    .with_quota(quota_module);

    // tenant-a is limited; tenant-b (no quota) is unlimited.
    m.create_network("tenant-a", &mut sample_net("a-1")).await.unwrap();
    assert!(m.create_network("tenant-a", &mut sample_net("a-2")).await.is_err());
    m.create_network("tenant-b", &mut sample_net("b-1")).await.unwrap();
    m.create_network("tenant-b", &mut sample_net("b-2")).await.unwrap();
}
