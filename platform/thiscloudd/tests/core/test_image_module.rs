use thiscloudd::image::{Image, ImageFormat, ImageModule, MemoryImageStore, MockImageBackend};

fn make_module() -> ImageModule {
    ImageModule::new(
        Box::new(MockImageBackend::default()),
        Box::new(MemoryImageStore::default()),
    )
}

#[tokio::test]
async fn test_register_generates_id_and_tenant() {
    let mut module = make_module();
    let mut img = Image::new("ubuntu".into(), "https://x/u.qcow2".into(), ImageFormat::Qcow2);
    let saved = module.register("t1", &mut img).await.unwrap();
    assert!(!saved.id.is_empty());
    assert_eq!(saved.tenant_id, "t1");
}

#[tokio::test]
async fn test_register_requires_name() {
    let mut module = make_module();
    let mut img = Image {
        id: String::new(),
        name: String::new(),
        source: "https://x/u.qcow2".into(),
        ..Image::new("x".into(), "https://x/u.qcow2".into(), ImageFormat::Qcow2)
    };
    img.name = String::new();
    let err = module.register("t1", &mut img).await.unwrap_err();
    assert!(err.to_string().contains("name is required"));
}

#[tokio::test]
async fn test_get_by_id_and_name() {
    let mut module = make_module();
    let mut img = Image::new("debian".into(), "https://x/d.qcow2".into(), ImageFormat::Qcow2);
    let saved = module.register("t1", &mut img).await.unwrap();
    let by_id = module.get("t1", &saved.id).await.unwrap();
    assert_eq!(by_id.name, "debian");
    let by_name = module.get_by_name("t1", "debian").await.unwrap();
    assert_eq!(by_name.unwrap().id, saved.id);
}

#[tokio::test]
async fn test_tenant_isolation() {
    let mut module = make_module();
    let mut img = Image::new("ubuntu".into(), "https://x/u.qcow2".into(), ImageFormat::Qcow2);
    module.register("t1", &mut img).await.unwrap();
    assert!(module.list("t2").await.unwrap().is_empty());
    assert_eq!(module.list("t1").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_set_template_and_remove() {
    let mut module = make_module();
    let mut img = Image::new("ubuntu".into(), "https://x/u.qcow2".into(), ImageFormat::Qcow2);
    let saved = module.register("t1", &mut img).await.unwrap();
    let tmpl = module.set_template("t1", &saved.id, true).await.unwrap();
    assert!(tmpl.template);
    module.remove("t1", &saved.id).await.unwrap();
    assert!(module.get("t1", &saved.id).await.is_err());
}

#[tokio::test]
async fn test_get_missing_errors() {
    let module = make_module();
    assert!(module.get("t1", "nope").await.is_err());
}