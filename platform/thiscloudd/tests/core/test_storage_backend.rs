use thiscloudd::storage::{
    CephBackend, LinstorBackend, MockStorageBackend, PoolType, StorageBackend, StoragePool,
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

#[test]
fn test_ceph_create_command() {
    let backend = CephBackend::new();
    let pool = sample_pool("rbd-pool");
    let cmd = backend.create_command(&pool);
    assert_eq!(
        cmd,
        vec![
            "ceph",
            "osd",
            "pool",
            "create",
            "rbd-pool",
            "32",
            "replicated"
        ]
    );
}

#[test]
fn test_ceph_pool_set_size_command() {
    let backend = CephBackend::new();
    let pool = sample_pool("rbd-pool");
    let cmd = backend.pool_set_size_command(&pool);
    assert_eq!(
        cmd,
        vec!["ceph", "osd", "pool", "set", "rbd-pool", "size", "2"]
    );
}

#[test]
fn test_ceph_application_enable_command() {
    let backend = CephBackend::new();
    let pool = sample_pool("rbd-pool");
    let cmd = backend.application_enable_command(&pool);
    assert_eq!(
        cmd,
        vec![
            "ceph",
            "osd",
            "pool",
            "application",
            "enable",
            "rbd-pool",
            "rbd"
        ]
    );
}

#[test]
fn test_ceph_delete_command() {
    let backend = CephBackend::new();
    let pool = sample_pool("rbd-pool");
    let cmd = backend.delete_command(&pool);
    assert_eq!(
        cmd,
        vec![
            "ceph",
            "osd",
            "pool",
            "rm",
            "rbd-pool",
            "rbd-pool",
            "--yes-i-really-really-mean-it"
        ]
    );
}

#[test]
fn test_ceph_pool_type_serde() {
    let pool = StoragePool::new(
        "tenant-a-vols".to_string(),
        PoolType::Ceph,
        Vec::new(),
        3,
    );
    let json = serde_json::to_string(&pool).unwrap();
    assert!(json.contains("\"pool_type\":\"ceph\""));
    let back: StoragePool = serde_json::from_str(&json).unwrap();
    assert_eq!(back.pool_type, PoolType::Ceph);
}
