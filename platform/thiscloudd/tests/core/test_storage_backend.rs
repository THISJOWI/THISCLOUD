use thiscloudd::storage::{
    LinstorBackend, MockStorageBackend, PoolType, StorageBackend, StoragePool,
};

fn sample_pool(name: &str) -> StoragePool {
    StoragePool::new(
        name.to_string(),
        PoolType::Linstor,
        vec!["/dev/sdb".to_string()],
        2,
    )
}

#[tokio::test]
async fn test_mock_backend_create_and_exists() {
    let backend = MockStorageBackend::new();
    let pool = sample_pool("data");
    backend.create(&pool).await.unwrap();
    assert!(backend.exists("data").await.unwrap());
    assert!(!backend.exists("nope").await.unwrap());
}

#[tokio::test]
async fn test_mock_backend_delete() {
    let backend = MockStorageBackend::new();
    let pool = sample_pool("data");
    backend.create(&pool).await.unwrap();
    backend.delete(&pool).await.unwrap();
    assert!(!backend.exists("data").await.unwrap());
}

#[test]
fn test_linstor_create_command() {
    let backend = LinstorBackend::new();
    let pool = sample_pool("data");
    let cmd = backend.create_command(&pool);
    assert_eq!(
        cmd,
        vec![
            "linstor",
            "storage-pool",
            "create",
            "file",
            "data",
            "--replicas=2"
        ]
    );
}

#[test]
fn test_linstor_delete_command() {
    let backend = LinstorBackend::new();
    let pool = sample_pool("data");
    let cmd = backend.delete_command(&pool);
    assert_eq!(cmd, vec!["linstor", "storage-pool", "delete", "data"]);
}
