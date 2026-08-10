use thiscloudd::marketplace::{AppType, MarketplaceApp, MarketplaceStatus};

#[test]
fn test_marketplace_app_serde_roundtrip() {
    let app = MarketplaceApp::new(
        "nginx".to_string(),
        AppType::DockerImage,
        "nginx:1.27".to_string(),
        "1.27".to_string(),
        "Web server".to_string(),
    );
    let json = serde_json::to_string(&app).unwrap();
    let back: MarketplaceApp = serde_json::from_str(&json).unwrap();
    assert_eq!(back, app);
}

#[test]
fn test_marketplace_app_defaults() {
    let app = MarketplaceApp::new(
        "alma-iso".to_string(),
        AppType::Iso,
        "https://repo/alma.iso".to_string(),
        "9".to_string(),
        "AlmaLinux ISO".to_string(),
    );
    assert!(!app.id.is_empty());
    assert_eq!(app.status, MarketplaceStatus::NotInstalled);
    assert_eq!(app.app_type.as_str(), "iso");
}

#[test]
fn test_app_type_str() {
    assert_eq!(AppType::Iso.as_str(), "iso");
    assert_eq!(AppType::DockerImage.as_str(), "docker");
    assert_eq!(AppType::CloudInit.as_str(), "cloud-init");
    assert_eq!(AppType::TurboKit.as_str(), "turbokit");
}
