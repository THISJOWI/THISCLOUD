use thiscloudd::storage::{MemoryStorageStore, PoolType, StoragePool, StorageStore};

fn sample_pool(name: &str) -> StoragePool {
    StoragePool::new(
        name.to_string(),
        PoolType::Linstor,
        vec!["/dev/sdb".to_string()],
        2,
    )
}

#[tokio::test]
async fn test_memory_store_put_get_list_delete() {
    let store = MemoryStorageStore::default();
    store.put(&sample_pool("data")).await.unwrap();
    store.put(&sample_pool("backup")).await.unwrap();

    let got = store.get("data").await.unwrap().unwrap();
    assert_eq!(got.name, "data");

    let all = store.list().await.unwrap();
    assert_eq!(all.len(), 2);

    store.delete("data").await.unwrap();
    assert!(store.get("data").await.unwrap().is_none());
    assert_eq!(store.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_memory_store_get_missing() {
    let store = MemoryStorageStore::default();
    assert!(store.get("missing").await.unwrap().is_none());
}
