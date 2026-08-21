use thiscloudd::network::{LogicalNetwork, MemoryNetworkStore, NetworkStore};

fn sample_net(id: &str) -> LogicalNetwork {
    LogicalNetwork::new(
        id.to_string(),
        id.to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    )
}

#[tokio::test]
async fn test_memory_store_put_get_list_delete() {
    let store = MemoryNetworkStore::default();
    store.put("", &sample_net("net-1")).await.unwrap();
    store.put("", &sample_net("net-2")).await.unwrap();

    let got = store.get("", "net-1").await.unwrap().unwrap();
    assert_eq!(got.name, "net-1");

    let all = store.list("").await.unwrap();
    assert_eq!(all.len(), 2);

    store.delete("", "net-1").await.unwrap();
    assert!(store.get("", "net-1").await.unwrap().is_none());
    assert_eq!(store.list("").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_memory_store_get_missing() {
    let store = MemoryNetworkStore::default();
    assert!(store.get("", "missing").await.unwrap().is_none());
}

// --- T0.4: multitenancy tenant isolation ---

#[tokio::test]
async fn test_network_store_tenant_isolation() {
    let store = MemoryNetworkStore::default();
    store
        .put("tenant-a", &sample_net("net-a"))
        .await
        .unwrap();
    store
        .put("tenant-b", &sample_net("net-b"))
        .await
        .unwrap();

    // Scoped list: tenant-a sees only its own resources.
    let a = store.list("tenant-a").await.unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].id, "net-a");

    let b = store.list("tenant-b").await.unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].id, "net-b");

    // Cross-tenant get returns None.
    assert!(store.get("tenant-b", "net-a").await.unwrap().is_none());

    // Global admin (empty tenant) sees all.
    let all = store.list("").await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_network_store_global_tenant_shares_keyspace() {
    let store = MemoryNetworkStore::default();
    store.put("", &sample_net("shared-1")).await.unwrap();
    store.put("", &sample_net("shared-2")).await.unwrap();
    let all = store.list("").await.unwrap();
    assert_eq!(all.len(), 2);
    // Scoped tenant cannot see global resources.
    assert!(store.get("tenant-x", "shared-1").await.unwrap().is_none());
}
