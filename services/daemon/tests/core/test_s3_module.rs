use thiscloudd::s3::{MemoryS3Store, MockS3Backend, S3Module};

fn make_module() -> S3Module {
    S3Module::new(
        Box::new(MockS3Backend::new()),
        Box::new(MemoryS3Store::default()),
    )
}

#[tokio::test]
async fn test_create_bucket_generates_id_and_tenant() {
    let mut module = make_module();
    let bucket = module.create_bucket("t1", "data").await.unwrap();
    assert!(!bucket.id.is_empty());
    assert_eq!(bucket.name, "data");
    assert_eq!(bucket.tenant_id, "t1");
}

#[tokio::test]
async fn test_create_bucket_requires_name() {
    let mut module = make_module();
    let err = module.create_bucket("t1", "").await.unwrap_err();
    assert!(err.to_string().contains("name is required"));
}

#[tokio::test]
async fn test_create_bucket_duplicate_conflict() {
    let mut module = make_module();
    module.create_bucket("t1", "data").await.unwrap();
    let err = module.create_bucket("t1", "data").await.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_get_bucket_by_name() {
    let mut module = make_module();
    module.create_bucket("t1", "data").await.unwrap();
    let bucket = module.get_bucket("t1", "data").await.unwrap();
    assert_eq!(bucket.name, "data");
    assert_eq!(bucket.tenant_id, "t1");
}

#[tokio::test]
async fn test_tenant_isolation() {
    let mut module = make_module();
    module.create_bucket("t1", "data").await.unwrap();
    assert!(module.list_buckets("t2").await.unwrap().is_empty());
    assert_eq!(module.list_buckets("t1").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_delete_bucket() {
    let mut module = make_module();
    module.create_bucket("t1", "data").await.unwrap();
    module.delete_bucket("t1", "data").await.unwrap();
    assert!(module.get_bucket("t1", "data").await.is_err());
    assert!(module.list_buckets("t1").await.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_missing_errors() {
    let module = make_module();
    assert!(module.get_bucket("t1", "nope").await.is_err());
}

#[tokio::test]
async fn test_issue_credentials_generates_keys() {
    let mut module = make_module();
    let key = module.issue_credentials("t1").await.unwrap();
    assert_eq!(key.access_key.len(), 20);
    assert_eq!(key.secret_key.len(), 40);
    assert_eq!(key.user, "thiscloud-t1");
    assert_eq!(key.tenant_id, "t1");
}

#[tokio::test]
async fn test_list_credentials_tenant_isolation() {
    let mut module = make_module();
    module.issue_credentials("t1").await.unwrap();
    module.issue_credentials("t1").await.unwrap();
    module.issue_credentials("t2").await.unwrap();
    assert_eq!(module.list_credentials("t1").await.unwrap().len(), 2);
    assert_eq!(module.list_credentials("t2").await.unwrap().len(), 1);
}