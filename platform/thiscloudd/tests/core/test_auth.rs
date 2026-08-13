//! T0.2 auth tests: login endpoint validation, JWT middleware enforcement,
//! and T0.4 multitenancy isolation through the HTTP layer.
//!
//! Real PAM/`su` authentication is intentionally not exercised here (it
//! requires an interactive system user); these tests cover the paths that run
//! before and around `su` plus full JWT enforcement.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use thiscloudd::auth::login::{router as login_router, LoginState};
use thiscloudd::auth::middleware::{encode_jwt, init_secret, jwt_auth};
use thiscloudd::auth::model::{Claims, Role};
use thiscloudd::compute::http::{app, ApiState};
use thiscloudd::compute::vm::VmConfig;
use thiscloudd::compute::{ComputeModule, MemoryVmStore, MockHypervisor};
use tokio::sync::Mutex;
use tower::ServiceExt;

const SECRET: &str = "test-secret";

/// Idempotent JWT secret init (OnceLock: first call wins, same secret used everywhere).
fn init() {
    init_secret(SECRET.to_string());
}

fn make_token(tenant_id: &str, exp: usize) -> String {
    let claims = Claims {
        sub: "user-1".to_string(),
        tenant_id: tenant_id.to_string(),
        role: Role::TenantUser,
        exp,
        iat: 1000000000,
    };
    encode_jwt(&claims, SECRET).unwrap()
}

fn login_state() -> LoginState {
    LoginState::new(SECRET.to_string(), 3600)
}

/// A minimal protected route to exercise the JWT middleware.
fn protected_router() -> Router {
    Router::new()
        .route("/ping", get(|| async { "pong" }))
        .layer(middleware::from_fn(jwt_auth))
}

// --- T0.2: login endpoint ---

#[tokio::test]
async fn test_login_missing_fields_400() {
    init();
    let router = axum::Router::new().nest("/api/v1", login_router(login_state()));

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"username":"","password":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_login_invalid_username_rejected() {
    init();
    let router = axum::Router::new().nest("/api/v1", login_router(login_state()));

    // Shell-injection-style username must be rejected before any auth command runs.
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"bob; rm -rf /","password":"x"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_oversized_username_rejected() {
    init();
    let router = axum::Router::new().nest("/api/v1", login_router(login_state()));

    let long_user = "a".repeat(33);
    let body = format!(r#"{{"username":"{long_user}","password":"x"}}"#);
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// --- T0.2: JWT middleware enforcement ---

#[tokio::test]
async fn test_jwt_missing_token_401() {
    init();
    let response = protected_router()
        .oneshot(
            Request::builder()
                .uri("/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_jwt_invalid_token_401() {
    init();
    let response = protected_router()
        .oneshot(
            Request::builder()
                .uri("/ping")
                .header("authorization", "Bearer garbage.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_jwt_expired_token_401() {
    init();
    let token = make_token("tenant-a", 1000000000); // expired (2001-09-09)
    let response = protected_router()
        .oneshot(
            Request::builder()
                .uri("/ping")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_jwt_valid_token_passes() {
    init();
    let token = make_token("tenant-a", 9999999999);
    let response = protected_router()
        .oneshot(
            Request::builder()
                .uri("/ping")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// --- T0.4: tenant isolation through HTTP (auth on, claims scoping) ---

#[tokio::test]
async fn test_http_tenant_isolation() {
    init();
    let module = ComputeModule::new(
        Box::new(MockHypervisor::new()),
        Box::new(MemoryVmStore::default()),
    );
    let router = axum::Router::new().nest(
        "/api/v1",
        app(ApiState::new(Arc::new(Mutex::new(module))))
            .layer(middleware::from_fn(jwt_auth)),
    );

    let vm_body = serde_json::to_string(&VmConfig::new(
        "vm-1".to_string(),
        "web1".to_string(),
        2,
        2048,
        "/var/lib/thiscloud/vms/web1.qcow2".to_string(),
        vec!["br0".to_string()],
    ))
    .unwrap();

    // tenant-a creates a VM.
    let token_a = make_token("tenant-a", 9999999999);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token_a}"))
                .body(Body::from(vm_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // tenant-b cannot see it.
    let token_b = make_token("tenant-b", 9999999999);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/vms")
                .header("authorization", format!("Bearer {token_b}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"[]");

    // tenant-a still sees it.
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/vms")
                .header("authorization", format!("Bearer {token_a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let vms: Vec<VmConfig> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(vms.len(), 1);
    assert_eq!(vms[0].tenant_id, "tenant-a");
}
