use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use thiscloudd::marketplace::http::{app, MarketplaceApiState};
use thiscloudd::marketplace::{MarketplaceModule, MemoryMarketplaceStore, MockMarketplaceBackend};
use tower::ServiceExt;

fn make_router() -> axum::Router {
    let module = MarketplaceModule::new(
        Box::new(MockMarketplaceBackend::default()),
        Box::new(MemoryMarketplaceStore::default()),
    );
    app(MarketplaceApiState::new(Arc::new(tokio::sync::Mutex::new(
        module,
    ))))
}

#[tokio::test]
async fn test_marketplace_http_install_and_list() {
    let router = make_router();

    let create_body = r#"{
        "name": "nginx",
        "app_type": "docker",
        "source": "nginx:latest",
        "version": "latest",
        "description": "web server"
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/marketplace/apps")
                .header("content-type", "application/json")
                .body(Body::from(create_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/marketplace/apps")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_marketplace_http_get_missing_404() {
    let router = make_router();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/marketplace/apps/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
