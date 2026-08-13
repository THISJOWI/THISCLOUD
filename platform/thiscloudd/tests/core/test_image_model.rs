use thiscloudd::image::{Image, ImageFormat, ImageStatus, OsFamily};

#[test]
fn test_image_defaults() {
    let img = Image::new("ubuntu".into(), "https://example.com/u.qcow2".into(), ImageFormat::Qcow2);
    assert!(!img.id.is_empty());
    assert_eq!(img.name, "ubuntu");
    assert_eq!(img.source, "https://example.com/u.qcow2");
    assert_eq!(img.format, ImageFormat::Qcow2);
    assert_eq!(img.os_family, OsFamily::Generic);
    assert_eq!(img.status, ImageStatus::Available);
    assert!(!img.template);
    assert_eq!(img.sha256, "");
    assert!(img.size_bytes == 0);
}

#[test]
fn test_image_serde_roundtrip() {
    let img = Image::new("iso".into(), "/pool/isos/a.iso".into(), ImageFormat::Iso);
    let json = serde_json::to_string(&img).unwrap();
    let back: Image = serde_json::from_str(&json).unwrap();
    assert_eq!(back, img);
}

#[test]
fn test_image_format_deserialize() {
    let v: ImageFormat = serde_json::from_str(r#""qcow2""#).unwrap();
    assert_eq!(v, ImageFormat::Qcow2);
    let v: ImageFormat = serde_json::from_str(r#""cloud-init""#).unwrap();
    assert_eq!(v, ImageFormat::CloudInit);
}

#[test]
fn test_image_status_serde() {
    assert_eq!(serde_json::to_string(&ImageStatus::Available).unwrap(), "\"available\"");
    assert_eq!(serde_json::to_string(&ImageStatus::Importing).unwrap(), "\"importing\"");
    assert_eq!(serde_json::to_string(&ImageStatus::Error).unwrap(), "\"error\"");
}