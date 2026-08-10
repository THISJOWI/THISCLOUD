use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use thiscloudd::compute::http::{app, ApiState};
use thiscloudd::compute::vm::VmConfig;
use thiscloudd::compute::{ComputeModule, MemoryVmStore, MockHypervisor};
use tokio::sync::Mutex;
use tower::ServiceExt;

fn build_state() -> ApiState {
    let module = ComputeModule::new(
        Box::new(MockHypervisor::new()),
        Box::new(MemoryVmStore::default()),
    );
    ApiState::new(Arc::new(Mutex::new(module)))
}

#[tokio::test]
async fn test_http_list_vms_empty() {
    let state = build_state();
    let app = app(state);

    let response = app
        .oneshot(Request::builder().uri("/vms").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"[]");
}

#[tokio::test]
async fn test_http_create_and_list_vms() {
    let state = build_state();
    let app = app(state);

    let body = serde_json::to_string(&VmConfig::new(
        "http-vm-1".to_string(),
        "web1".to_string(),
        2,
        2048,
        "/var/lib/thiscloud/vms/web1.qcow2".to_string(),
        vec!["br0".to_string()],
    ))
    .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vms")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(Request::builder().uri("/vms").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let vms: Vec<VmConfig> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(vms.len(), 1);
    assert_eq!(vms[0].id, "http-vm-1");
}

#[tokio::test]
async fn test_http_get_vm() {
    let state = build_state();
    let app = app(state);

    let body = serde_json::to_string(&VmConfig::new(
        "http-vm-2".to_string(),
        "db1".to_string(),
        4,
        4096,
        "/var/lib/thiscloud/vms/db1.qcow2".to_string(),
        vec!["br0".to_string()],
    ))
    .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vms")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/vms/http-vm-2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let vm: VmConfig = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(vm.name, "db1");
    assert_eq!(vm.cpus, 4);
}

#[tokio::test]
async fn test_http_start_stop_vm() {
    let state = build_state();
    let app = app(state);

    let body = serde_json::to_string(&VmConfig::new(
        "http-vm-3".to_string(),
        "web2".to_string(),
        1,
        1024,
        "/var/lib/thiscloud/vms/web2.qcow2".to_string(),
        vec!["br0".to_string()],
    ))
    .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vms")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vms/http-vm-3/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/vms/http-vm-3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let vm: VmConfig = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(vm.status, thiscloudd::compute::vm::VmStatus::Running);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vms/http-vm-3/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/vms/http-vm-3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let vm: VmConfig = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(vm.status, thiscloudd::compute::vm::VmStatus::Stopped);
}

#[tokio::test]
async fn test_http_delete_vm() {
    let state = build_state();
    let app = app(state);

    let body = serde_json::to_string(&VmConfig::new(
        "http-vm-4".to_string(),
        "del1".to_string(),
        1,
        1024,
        "/var/lib/thiscloud/vms/del1.qcow2".to_string(),
        vec!["br0".to_string()],
    ))
    .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vms")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/vms/http-vm-4")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/vms/http-vm-4")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_http_get_missing_vm_404() {
    let state = build_state();
    let app = app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/vms/ghost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
