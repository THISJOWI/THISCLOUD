use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use thiscloudd::image::http::{app, ImageApiState};
use thiscloudd::image::{ImageModule, MemoryImageStore, MockImageBackend};
use tower::ServiceExt;

fn make_router() -> axum::Router {
    let module = ImageModule::new(
        Box::new(MockImageBackend::default()),
        Box::new(MemoryImageStore::default()),
    );
    axum::Router::new().nest(
        "/api/v1",
        app(ImageApiState::new(Arc::new(tokio::sync::Mutex::new(
            module,
        )))),
    )
}

#[tokio::test]
async fn test_image_http_register_and_list() {
    let router = make_router();

    let create_body = r#"{
        "name": "ubuntu",
        "source": "https://example.com/u.qcow2",
        "format": "qcow2",
        "os_family": "ubuntu",
        "version": "24.04"
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/images")
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
                .uri("/api/v1/images")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_image_http_get_missing_404() {
    let router = make_router();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/images/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_image_http_template_and_delete() {
    let router = make_router();

    let create_body = r#"{
        "name": "rocky",
        "source": "https://example.com/r.qcow2",
        "format": "raw"
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/images")
                .header("content-type", "application/json")
                .body(Body::from(create_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = created["id"].as_str().unwrap().to_string();

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/images/{id}/template"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"template": true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/images/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}