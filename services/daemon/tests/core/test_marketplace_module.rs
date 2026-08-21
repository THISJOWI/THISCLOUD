use thiscloudd::marketplace::{
    AppType, MarketplaceApp, MarketplaceModule, MemoryMarketplaceStore, MockMarketplaceBackend,
};

fn module() -> MarketplaceModule {
    MarketplaceModule::new(
        Box::new(MockMarketplaceBackend::default()),
        Box::new(MemoryMarketplaceStore::default()),
    )
}

fn app(name: &str) -> MarketplaceApp {
    MarketplaceApp::new(
        name.to_string(),
        AppType::DockerImage,
        format!("{name}:latest"),
        "latest".to_string(),
        "test".to_string(),
    )
}

#[tokio::test]
async fn test_marketplace_install_and_list() {
    let mut m = module();
    let mut a = app("nginx");
    let installed = m.install("", &mut a).await.unwrap();
    assert_eq!(
        installed.status,
        thiscloudd::marketplace::MarketplaceStatus::Installed
    );
    assert_eq!(m.list("").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_marketplace_get() {
    let mut m = module();
    let mut a = app("nginx");
    let installed = m.install("", &mut a).await.unwrap();
    let got = m.get("", &installed.id).await.unwrap();
    assert_eq!(got.name, "nginx");
}

#[tokio::test]
async fn test_marketplace_get_missing_errors() {
    let m = module();
    assert!(m.get("", "nope").await.is_err());
}

#[tokio::test]
async fn test_marketplace_uninstall() {
    let mut m = module();
    let mut a = app("nginx");
    let installed = m.install("", &mut a).await.unwrap();
    m.uninstall("", &installed.id).await.unwrap();
    assert!(m.get("", &installed.id).await.is_err());
}

#[tokio::test]
async fn test_marketplace_install_empty_name_errors() {
    let mut m = module();
    let mut a = app("");
    assert!(m.install("", &mut a).await.is_err());
}
