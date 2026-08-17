use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use thiscloudd::core::Daemon;
use tower::ServiceExt;

#[tokio::test]
async fn test_healthz_returns_ok() {
    let config = thiscloudd::config::ThisCloudConfig::default();
    let daemon = Daemon::new(config, None);

    let response = daemon
        .http_router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_ready_without_etcd_reports_ready() {
    // In-memory stores (no etcd) have no external dependency: the daemon is
    // ready as soon as it serves. This is the dev/single-node path.
    let config = thiscloudd::config::ThisCloudConfig::default();
    let daemon = Daemon::new(config, None);

    let response = daemon
        .http_router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "ready");
    assert_eq!(body["checks"]["etcd"], true);
}