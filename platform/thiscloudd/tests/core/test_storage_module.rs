use thiscloudd::core::Module;
use thiscloudd::storage::{
    MemoryStorageStore, MockStorageBackend, PoolType, StorageModule, StoragePool,
};

fn sample_pool(name: &str) -> StoragePool {
    StoragePool::new(
        name.to_string(),
        PoolType::Linstor,
        vec!["/dev/sdb".to_string()],
        2,
    )
}

fn module() -> StorageModule {
    StorageModule::new(
        Box::new(MockStorageBackend::new()),
        Box::new(MemoryStorageStore::default()),
    )
}

#[tokio::test]
async fn test_storage_module_create_and_list() {
    let mut m = module();
    m.create_pool("", sample_pool("data")).await.unwrap();
    m.create_pool("", sample_pool("backup")).await.unwrap();
    assert_eq!(m.list_pools("").await.unwrap().len(), 2);
}

#[tokio::test]
async fn test_storage_module_get() {
    let mut m = module();
    m.create_pool("", sample_pool("data")).await.unwrap();
    let pool = m.get_pool("", "data").await.unwrap();
    assert_eq!(pool.name, "data");
    assert_eq!(pool.replication, 2);
}

#[tokio::test]
async fn test_storage_module_get_missing_errors() {
    let m = module();
    assert!(m.get_pool("", "nope").await.is_err());
}

#[tokio::test]
async fn test_storage_module_duplicate_name_errors() {
    let mut m = module();
    m.create_pool("", sample_pool("data")).await.unwrap();
    let err = m.create_pool("", sample_pool("data")).await.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_storage_module_delete() {
    let mut m = module();
    m.create_pool("", sample_pool("data")).await.unwrap();
    m.delete_pool("", "data").await.unwrap();
    assert!(m.list_pools("").await.unwrap().is_empty());
}

#[tokio::test]
async fn test_storage_module_name() {
    assert_eq!(module().name(), "storage");
}
