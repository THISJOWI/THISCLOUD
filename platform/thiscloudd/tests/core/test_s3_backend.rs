use thiscloudd::s3::{MockS3Backend, RadosgwBackend, S3Backend, S3Bucket};

fn sample_bucket(name: &str, tenant_id: &str) -> S3Bucket {
    let mut bucket = S3Bucket::new(name.to_string());
    bucket.tenant_id = tenant_id.to_string();
    bucket
}

#[tokio::test]
async fn test_mock_backend_create_user_and_bucket_exists() {
    let backend = MockS3Backend::new();
    backend.create_user("t1").await.unwrap();
    let bucket = sample_bucket("data", "t1");
    backend.create_bucket(&bucket).await.unwrap();
    assert!(backend.exists_bucket("data").await.unwrap());
    assert!(!backend.exists_bucket("nope").await.unwrap());
}

#[tokio::test]
async fn test_mock_backend_delete_bucket() {
    let backend = MockS3Backend::new();
    let bucket = sample_bucket("data", "t1");
    backend.create_bucket(&bucket).await.unwrap();
    backend.delete_bucket(&bucket).await.unwrap();
    assert!(!backend.exists_bucket("data").await.unwrap());
}

#[test]
fn test_radosgw_create_user_command() {
    let backend = RadosgwBackend::new("http://127.0.0.1:7480".into());
    let cmd = backend.create_user_command("t1");
    assert_eq!(
        cmd,
        vec![
            "radosgw-admin",
            "user",
            "create",
            "--uid=thiscloud-t1",
            "--display-name=thiscloud"
        ]
    );
}

#[test]
fn test_radosgw_create_subuser_command() {
    let backend = RadosgwBackend::new("http://127.0.0.1:7480".into());
    let cmd = backend.create_subuser_command("t1");
    assert_eq!(
        cmd,
        vec![
            "radosgw-admin",
            "subuser",
            "create",
            "--uid=thiscloud-t1",
            "--subuser=thiscloud-t1:swift",
            "--access=full"
        ]
    );
}

#[test]
fn test_radosgw_create_bucket_command() {
    let backend = RadosgwBackend::new("http://127.0.0.1:7480".into());
    let bucket = sample_bucket("data", "t1");
    let cmd = backend.create_bucket_command(&bucket);
    assert_eq!(
        cmd,
        vec![
            "radosgw-admin",
            "bucket",
            "create",
            "--bucket=data",
            "--uid=thiscloud-t1"
        ]
    );
}

#[test]
fn test_radosgw_delete_bucket_command() {
    let backend = RadosgwBackend::new("http://127.0.0.1:7480".into());
    let bucket = sample_bucket("data", "t1");
    let cmd = backend.delete_bucket_command(&bucket);
    assert_eq!(
        cmd,
        vec![
            "radosgw-admin",
            "bucket",
            "rm",
            "--bucket=data",
            "--purge-objects"
        ]
    );
}

#[test]
fn test_radosgw_bucket_stats_command() {
    let backend = RadosgwBackend::new("http://127.0.0.1:7480".into());
    let cmd = backend.bucket_stats_command("data");
    assert_eq!(
        cmd,
        vec!["radosgw-admin", "bucket", "stats", "--bucket=data"]
    );
}