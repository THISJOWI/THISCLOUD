use thiscloudd::marketplace::{
    AppType, DockerHubBackend, MarketplaceApp, MarketplaceBackend, MockMarketplaceBackend,
};

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
async fn test_mock_backend_install_exists_uninstall() {
    let backend = MockMarketplaceBackend::default();
    let a = app("nginx");
    assert!(!backend.exists(&a.name).await.unwrap());
    backend.install(&a).await.unwrap();
    assert!(backend.exists(&a.name).await.unwrap());
    backend.uninstall(&a).await.unwrap();
    assert!(!backend.exists(&a.name).await.unwrap());
}

#[test]
fn test_docker_hub_install_command_docker() {
    let a = MarketplaceApp::new(
        "redis".to_string(),
        AppType::DockerImage,
        "redis:7".to_string(),
        "7".to_string(),
        "cache".to_string(),
    );
    assert_eq!(
        DockerHubBackend::install_command(&a),
        vec!["docker", "pull", "redis:7"]
    );
}

#[test]
fn test_docker_hub_install_command_turbokit() {
    let a = MarketplaceApp::new(
        "bundle".to_string(),
        AppType::TurboKit,
        "repo/bundle.tk".to_string(),
        "1".to_string(),
        "tk".to_string(),
    );
    assert_eq!(
        DockerHubBackend::install_command(&a),
        vec!["turbokit", "install", "repo/bundle.tk"]
    );
}
