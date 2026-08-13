//! T0.3 RBAC tests: role-based access enforcement through the HTTP layer.
//!
//! Each test builds a minimal resource router (with JWT auth + RBAC layers)
//! and sends requests with different role tokens to verify access control.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::Router;
use std::sync::Arc;
use thiscloudd::auth::middleware::{encode_jwt, init_secret, jwt_auth};
use thiscloudd::auth::model::{Claims, Role};
use thiscloudd::compute::http::{app, ApiState};
use thiscloudd::compute::vm::VmConfig;
use thiscloudd::compute::{ComputeModule, MemoryVmStore, MockHypervisor};
use tokio::sync::Mutex;
use tower::ServiceExt;

// Shared secret — MUST match test_auth.rs since JWT_SECRET is process-global (OnceLock).
const SECRET: &str = "test-secret";

fn init() {
    init_secret(SECRET.to_string());
}

fn make_token(role: Role, tenant_id: &str) -> String {
    let claims = Claims {
        sub: "user-1".to_string(),
        tenant_id: tenant_id.to_string(),
        role,
        exp: 9999999999,
        iat: 1000000000,
    };
    encode_jwt(&claims, SECRET).unwrap()
}

fn vm_body(name: &str) -> String {
    serde_json::to_string(&VmConfig::new(
        format!("vm-{name}"),
        name.to_string(),
        2,
        2048,
        format!("/tmp/{name}.qcow2"),
        vec!["br0".to_string()],
    ))
    .unwrap()
}

/// Compute router with JWT auth + RBAC layers (mirrors production setup).
fn router() -> Router {
    let module = ComputeModule::new(
        Box::new(MockHypervisor::new()),
        Box::new(MemoryVmStore::default()),
    );
    axum::Router::new().nest(
        "/api/v1",
        app(ApiState::new(Arc::new(Mutex::new(module))))
            .layer(middleware::from_fn(jwt_auth)),
    )
}

// ---------------------------------------------------------------------------
// Create (POST /vms) — CreateSet = Admin, Operator, TenantAdmin, TenantUser
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tenant_user_create_vm_201() {
    init();
    let token = make_token(Role::TenantUser, "tenant-a");
    let resp = router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(vm_body("web1")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn auditor_create_vm_403() {
    init();
    let token = make_token(Role::Auditor, "tenant-a");
    let resp = router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(vm_body("blocked")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// Read (GET /vms) — ReadSet = all roles
// ---------------------------------------------------------------------------

#[tokio::test]
async fn read_routes_allow_all_roles() {
    init();
    for role in [
        Role::Admin,
        Role::Operator,
        Role::TenantAdmin,
        Role::TenantUser,
        Role::Auditor,
    ] {
        let r = router();
        // Create a VM as admin first (so the list isn't empty).
        let admin_token = make_token(Role::Admin, "tenant-a");
        r.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/vms")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::from(vm_body("r1")))
                    .unwrap(),
            )
            .await
            .unwrap();

        let token = make_token(role.clone(), "tenant-a");
        let resp = r
            .oneshot(
                Request::builder()
                    .uri("/api/v1/vms")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "{role:?} should be able to list vms");
    }
}

// ---------------------------------------------------------------------------
// Delete (DELETE /vms/:id) — WriteSet = Admin, Operator, TenantAdmin
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_delete_vm_200() {
    init();
    let r = router();
    let admin = make_token(Role::Admin, "tenant-a");
    // Create
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin}"))
                .body(Body::from(vm_body("del1")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let vm: VmConfig = serde_json::from_slice(&bytes).unwrap();

    // Delete
    let resp = r
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/vms/{}", vm.id))
                .header("authorization", format!("Bearer {admin}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn tenant_user_delete_vm_403() {
    init();
    let r = router();
    let admin = make_token(Role::Admin, "tenant-a");
    // Create as admin
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin}"))
                .body(Body::from(vm_body("del2")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let vm: VmConfig = serde_json::from_slice(&bytes).unwrap();

    // Attempt delete as tenant_user → 403
    let tu = make_token(Role::TenantUser, "tenant-a");
    let resp = r
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/vms/{}", vm.id))
                .header("authorization", format!("Bearer {tu}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn auditor_delete_vm_403() {
    init();
    let r = router();
    let admin = make_token(Role::Admin, "tenant-a");
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin}"))
                .body(Body::from(vm_body("del3")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let vm: VmConfig = serde_json::from_slice(&bytes).unwrap();

    let auditor = make_token(Role::Auditor, "tenant-a");
    let resp = r
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/vms/{}", vm.id))
                .header("authorization", format!("Bearer {auditor}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// Start/Stop — WriteSet = Admin, Operator, TenantAdmin
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tenant_user_start_vm_403() {
    init();
    let r = router();
    let admin = make_token(Role::Admin, "tenant-a");
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin}"))
                .body(Body::from(vm_body("s1")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let vm: VmConfig = serde_json::from_slice(&bytes).unwrap();

    let tu = make_token(Role::TenantUser, "tenant-a");
    let resp = r
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/vms/{}/start", vm.id))
                .header("authorization", format!("Bearer {tu}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// No auth context → bypass (dev mode)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_auth_context_bypasses_rbac() {
    // No jwt_auth layer applied — same as THISCLOUD_AUTH_DISABLED=1.
    let module = ComputeModule::new(
        Box::new(MockHypervisor::new()),
        Box::new(MemoryVmStore::default()),
    );
    let r = app(ApiState::new(Arc::new(Mutex::new(module))));

    // DELETE without any auth → should reach handler (404, not 403).
    let resp = r
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/vms/ghost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
