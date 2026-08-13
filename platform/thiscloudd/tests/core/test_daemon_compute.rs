use thiscloudd::config::ThisCloudConfig;
use tower::ServiceExt;

#[tokio::test]
async fn test_daemon_registers_compute_module() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    assert_eq!(daemon.module_count().await, 8);
    assert_eq!(
        daemon.module_names().await,
        vec![
            "compute",
            "network",
            "storage",
            "marketplace",
            "node",
            "image",
            "s3",
            "metrics"
        ]
    );
}

#[tokio::test]
async fn test_daemon_serves_http_api() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    let app = daemon.http_router();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/vms")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn test_daemon_serves_network_and_compute_routes() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    let app = daemon.http_router();
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/networks")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/vms")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn test_daemon_serves_storage_pools_route() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    let app = daemon.http_router();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/storage/pools")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn test_daemon_serves_marketplace_route() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    let app = daemon.http_router();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/marketplace/apps")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn test_daemon_compute_binds_configured_port() {
    let mut config = ThisCloudConfig::default();
    config.compute.http_port = 8090;
    let daemon = thiscloudd::core::Daemon::new(config);

    assert_eq!(daemon.http_bind(), "127.0.0.1");
    assert_eq!(daemon.http_port(), 8090);
}

#[tokio::test]
async fn test_daemon_unversioned_paths_404() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    let app = daemon.http_router();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/vms")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_daemon_serves_openapi_contract() {
    let config = ThisCloudConfig::default();
    let daemon = thiscloudd::core::Daemon::new(config);

    let app = daemon.http_router();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/openapi.yaml")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap(),
        "application/yaml"
    );
}
