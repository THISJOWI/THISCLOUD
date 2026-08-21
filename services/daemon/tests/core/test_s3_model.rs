use thiscloudd::s3::{S3AccessKey, S3Bucket};

#[test]
fn test_s3_bucket_defaults() {
    let bucket = S3Bucket::new("data".into());
    assert!(!bucket.id.is_empty());
    assert_eq!(bucket.name, "data");
    assert_eq!(bucket.tenant_id, "");
    assert_eq!(bucket.created_at, "");
}

#[test]
fn test_s3_bucket_serde_roundtrip() {
    let bucket = S3Bucket::new("backups".into());
    let json = serde_json::to_string(&bucket).unwrap();
    let back: S3Bucket = serde_json::from_str(&json).unwrap();
    assert_eq!(back, bucket);
}

#[test]
fn test_s3_bucket_deserialize_missing_fields() {
    let bucket: S3Bucket = serde_json::from_str(r#"{"name":"data"}"#).unwrap();
    assert_eq!(bucket.name, "data");
    assert_eq!(bucket.id, "");
    assert_eq!(bucket.tenant_id, "");
    assert_eq!(bucket.created_at, "");
}

#[test]
fn test_s3_access_key_serde_roundtrip() {
    let key = S3AccessKey::new(
        "AKIA1234567890".into(),
        "secret1234567890".into(),
        "thiscloud-t1".into(),
    );
    let json = serde_json::to_string(&key).unwrap();
    let back: S3AccessKey = serde_json::from_str(&json).unwrap();
    assert_eq!(back, key);
    assert_eq!(back.tenant_id, "");
}