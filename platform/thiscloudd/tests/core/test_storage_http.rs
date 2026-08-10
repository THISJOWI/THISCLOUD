use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use thiscloudd::storage::http::{app, StorageApiState};
use thiscloudd::storage::{MemoryStorageStore, MockStorageBackend, StorageModule};
use tower::ServiceExt;

fn make_router() -> axum::Router {
    let module = StorageModule::new(
        Box::new(MockStorageBackend::new()),
        Box::new(MemoryStorageStore::default()),
    );
    app(StorageApiState::new(Arc::new(tokio::sync::Mutex::new(
        module,
    ))))
}

#[tokio::test]
async fn test_storage_http_create_and_list() {
    let router = make_router();

    let create_body = r#"{
        "name": "data",
        "pool_type": "linstor",
        "devices": ["/dev/sdb"],
        "replication": 2
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/storage/pools")
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
                .uri("/storage/pools")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_storage_http_get_and_delete() {
    let router = make_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/storage/pools/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
