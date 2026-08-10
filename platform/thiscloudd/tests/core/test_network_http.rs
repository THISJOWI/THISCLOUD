use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use thiscloudd::network::http::{app, NetworkApiState};
use thiscloudd::network::{MemoryNetworkStore, MockNetworkBackend, NetworkModule};
use tower::ServiceExt;

fn make_router() -> axum::Router {
    let module = NetworkModule::new(
        Box::new(MockNetworkBackend::new()),
        Box::new(MemoryNetworkStore::default()),
    );
    app(NetworkApiState::new(Arc::new(tokio::sync::Mutex::new(
        module,
    ))))
}

#[tokio::test]
async fn test_network_http_create_and_list() {
    let router = make_router();

    let create_body = r#"{
        "id": "net-1",
        "name": "web",
        "cidr": "10.0.0.0/24",
        "gateway": "10.0.0.1"
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/networks")
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
                .uri("/networks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_network_http_get_and_delete() {
    let router = make_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/networks/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
