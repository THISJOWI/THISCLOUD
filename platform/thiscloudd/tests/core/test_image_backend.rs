use thiscloudd::image::{Image, ImageBackend, ImageFormat, MockImageBackend};

#[tokio::test]
async fn test_mock_backend_import_and_exists() {
    let backend = MockImageBackend::default();
    let img = Image::new("test".into(), "https://example.com/t.qcow2".into(), ImageFormat::Qcow2);
    backend.import(&img).await.unwrap();
    // Mock backend records imports; remove clears them.
    backend.remove(&img).await.unwrap();
}

#[tokio::test]
async fn test_mock_backend_import_cloud_init_noop() {
    let backend = MockImageBackend::default();
    let img = Image::new("ci".into(), "#cloud-config\n".into(), ImageFormat::CloudInit);
    backend.import(&img).await.unwrap();
    backend.remove(&img).await.unwrap();
}