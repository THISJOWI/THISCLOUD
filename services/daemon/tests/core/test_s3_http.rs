use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use thiscloudd::s3::http::{app, S3ApiState};
use thiscloudd::s3::{MemoryS3Store, MockS3Backend, S3Module};
use tower::ServiceExt;

fn make_router() -> axum::Router {
    let module = S3Module::new(
        Box::new(MockS3Backend::new()),
        Box::new(MemoryS3Store::default()),
    );
    axum::Router::new().nest(
        "/api/v1",
        app(S3ApiState::new(Arc::new(tokio::sync::Mutex::new(
            module,
        )))),
    )
}

#[tokio::test]
async fn test_s3_http_create_and_list_buckets() {
    let router = make_router();

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/s3/buckets")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"data"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/s3/buckets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_s3_http_get_missing_404() {
    let router = make_router();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/s3/buckets/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_s3_http_create_empty_name_400() {
    let router = make_router();
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/s3/buckets")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_s3_http_delete_bucket() {
    let router = make_router();

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/s3/buckets")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"data"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/s3/buckets/data")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/s3/buckets/data")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_s3_http_issue_and_list_credentials() {
    let router = make_router();

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/s3/credentials")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let key: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!key["access_key"].as_str().unwrap().is_empty());
    assert!(!key["secret_key"].as_str().unwrap().is_empty());

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/s3/credentials")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let keys: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(keys.as_array().unwrap().len(), 1);
}