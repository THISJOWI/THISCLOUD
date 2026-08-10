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
    store.put(&sample_net("net-1")).await.unwrap();
    store.put(&sample_net("net-2")).await.unwrap();

    let got = store.get("net-1").await.unwrap().unwrap();
    assert_eq!(got.name, "net-1");

    let all = store.list().await.unwrap();
    assert_eq!(all.len(), 2);

    store.delete("net-1").await.unwrap();
    assert!(store.get("net-1").await.unwrap().is_none());
    assert_eq!(store.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_memory_store_get_missing() {
    let store = MemoryNetworkStore::default();
    assert!(store.get("missing").await.unwrap().is_none());
}
