use thiscloudd::storage::{PoolType, StoragePool};

#[test]
fn test_storage_pool_serde_roundtrip() {
    let pool = StoragePool::new(
        "data".to_string(),
        PoolType::Linstor,
        vec!["/dev/sdb".to_string()],
        2,
    );
    let json = serde_json::to_string(&pool).unwrap();
    let parsed: StoragePool = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, pool);
}

#[test]
fn test_storage_pool_defaults() {
    let pool = StoragePool::new(
        "data".to_string(),
        PoolType::Linstor,
        vec!["/dev/sdb".to_string()],
        2,
    );
    assert_eq!(pool.name, "data");
    assert_eq!(pool.replication, 2);
    assert!(pool.pool_type == PoolType::Linstor);
}

#[test]
fn test_pool_type_serde() {
    assert_eq!(
        serde_json::to_string(&PoolType::Linstor).unwrap(),
        "\"linstor\""
    );
}
