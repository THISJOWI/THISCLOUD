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
    let app = axum::Router::new().nest("/api/v1", app(state));

    let response = app
        .oneshot(Request::builder().uri("/api/v1/vms").body(Body::empty()).unwrap())
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
    let app = axum::Router::new().nest("/api/v1", app(state));

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
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(Request::builder().uri("/api/v1/vms").body(Body::empty()).unwrap())
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
    let app = axum::Router::new().nest("/api/v1", app(state));

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
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/vms/http-vm-2")
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
    let app = axum::Router::new().nest("/api/v1", app(state));

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
                .uri("/api/v1/vms")
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
                .uri("/api/v1/vms/http-vm-3/start")
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
                .uri("/api/v1/vms/http-vm-3")
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
                .uri("/api/v1/vms/http-vm-3/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/vms/http-vm-3")
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
    let app = axum::Router::new().nest("/api/v1", app(state));

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
                .uri("/api/v1/vms")
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
                .uri("/api/v1/vms/http-vm-4")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/vms/http-vm-4")
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
    let app = axum::Router::new().nest("/api/v1", app(state));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/vms/ghost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// --- T0.5: quota enforcement via HTTP (409 Conflict) ---

#[tokio::test]
async fn test_http_quota_violation_returns_409() {
    use axum::middleware;
    use thiscloudd::auth::middleware::{encode_jwt, init_secret, jwt_auth};
    use thiscloudd::auth::model::{Claims, Role};
    use thiscloudd::quota::http::{app as quota_app, QuotaApiState};
    use thiscloudd::quota::QuotaModule;

    const SECRET: &str = "quota-test-secret";
    init_secret(SECRET.to_string());

    let token = encode_jwt(
        &Claims {
            sub: "user-1".into(),
            tenant_id: "tenant-a".into(),
            role: Role::TenantUser,
            exp: 9999999999,
            iat: 1000000000,
        },
        SECRET,
    )
    .unwrap();

    let quota_module = Arc::new(Mutex::new(QuotaModule::with_memory_store()));

    // Compute router with quota enforcement wired in + JWT auth (production shape).
    let module = ComputeModule::new(
        Box::new(MockHypervisor::new()),
        Box::new(MemoryVmStore::default()),
    )
    .with_quota(quota_module.clone());
    let resource_router = app(ApiState::new(Arc::new(Mutex::new(module))))
        .layer(middleware::from_fn(jwt_auth))
        .merge(quota_app(QuotaApiState::new(quota_module)));

    // Production shape: everything nested under /api/v1.
    let full = axum::Router::new().nest("/api/v1", resource_router);

    // Set max_vms=1 for tenant-a.
    let set_quota = serde_json::json!({
        "tenant_id": "tenant-a",
        "max_vms": 1,
        "max_cpus": 16,
        "max_memory_mb": 65536,
        "max_storage_gb": 0,
        "max_networks": 0
    });
    let resp = full
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/quotas/tenant-a")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(set_quota.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // First VM: within quota.
    let body = serde_json::to_string(&VmConfig::new(
        "quota-vm-1".to_string(),
        "q1".to_string(),
        1,
        1024,
        "/var/lib/thiscloud/vms/q1.qcow2".to_string(),
        vec!["br0".to_string()],
    ))
    .unwrap();
    let resp = full
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Second VM: exceeds max_vms=1 → 409 Conflict.
    let body = serde_json::to_string(&VmConfig::new(
        "quota-vm-2".to_string(),
        "q2".to_string(),
        1,
        1024,
        "/var/lib/thiscloud/vms/q2.qcow2".to_string(),
        vec!["br0".to_string()],
    ))
    .unwrap();
    let resp = full
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
