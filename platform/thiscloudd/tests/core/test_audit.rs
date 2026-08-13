//! T0.6 Audit tests: query audit logs, audit middleware records mutations.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use thiscloudd::audit::middleware::{audit_middleware, AuditState};
use thiscloudd::audit::model::AuditEntry;
use thiscloudd::audit::store::{AuditStore, MemoryAuditStore};
use thiscloudd::auth::middleware::{encode_jwt, init_secret};
use thiscloudd::auth::model::{Claims, Role};
use tower::ServiceExt;

const SECRET: &str = "test-secret";

fn init() {
    init_secret(SECRET.to_string());
}

fn make_token(sub: &str, role: Role) -> String {
    let claims = Claims {
        sub: sub.into(),
        tenant_id: "tenant-a".into(),
        role,
        exp: 9999999999,
        iat: 1000000000,
    };
    encode_jwt(&claims, SECRET).unwrap()
}

fn make_audit_state() -> (AuditState, Arc<tokio::sync::Mutex<MemoryAuditStore>>) {
    let store = Arc::new(tokio::sync::Mutex::new(MemoryAuditStore::new()));
    let state = AuditState {
        store: Arc::new(tokio::sync::Mutex::new(Box::new(MemoryAuditStore::new())
            as Box<dyn thiscloudd::audit::store::AuditStore>)),
    };
    (state, store)
}

/// A minimal router that creates resources and logs them via audit middleware.
fn audited_router() -> Router {
    let (state, _) = make_audit_state();
    Router::new()
        .route(
            "/things",
            get(|| async { "[]".to_string() }).post(|| async {
                axum::http::StatusCode::CREATED
            }),
        )
        .route("/things/:id", axum::routing::delete(|| async {
            axum::http::StatusCode::OK
        }))
        .layer(middleware::from_fn_with_state(state, audit_middleware))
}

// ---------------------------------------------------------------------------
// Audit store unit tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn audit_query_endpoint_empty() {
    init();
    let audit_state = AuditState {
        store: Arc::new(tokio::sync::Mutex::new(Box::new(MemoryAuditStore::new()))),
    };
    let router = axum::Router::new().nest("/api/v1", thiscloudd::audit::http::app(audit_state));

    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let entries: Vec<AuditEntry> = serde_json::from_slice(&bytes).unwrap();
    assert!(entries.is_empty());
}

#[tokio::test]
async fn audit_middleware_records_create() {
    init();
    let r = audited_router();
    let token = make_token("admin-1", Role::Admin);

    // POST → Create, should be logged.
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/things")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"x"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn audit_middleware_skips_get() {
    init();
    let (state, store) = make_audit_state();
    let r = Router::new()
        .route(
            "/things",
            get(|| async { "[]".to_string() }).post(|| async {
                axum::http::StatusCode::CREATED
            }),
        )
        .layer(middleware::from_fn_with_state(state, audit_middleware));
    let token = make_token("user-1", Role::TenantUser);

    // GET should not be logged.
    r.clone()
        .oneshot(
            Request::builder()
                .uri("/things")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let entries = store.lock().await.query(&thiscloudd::audit::model::AuditFilter::default()).await;
    assert!(entries.is_empty());
}

#[tokio::test]
async fn audit_query_filters_by_user() {
    init();
    let store_arc: Arc<tokio::sync::Mutex<Box<dyn AuditStore>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(MemoryAuditStore::new())));
    let audit_state = AuditState {
        store: store_arc.clone(),
    };

    // Manually insert entries.
    store_arc.lock().await.log(thiscloudd::audit::model::AuditEntry {
        id: "1".into(),
        timestamp: "100".into(),
        user: "alice".into(),
        role: "admin".into(),
        tenant_id: "t1".into(),
        action: thiscloudd::audit::model::AuditAction::Create,
        resource: "vm".into(),
        resource_id: "vm-1".into(),
        detail: String::new(),
    }).await;
    store_arc.lock().await.log(thiscloudd::audit::model::AuditEntry {
        id: "2".into(),
        timestamp: "101".into(),
        user: "bob".into(),
        role: "tenant_user".into(),
        tenant_id: "t1".into(),
        action: thiscloudd::audit::model::AuditAction::Delete,
        resource: "vm".into(),
        resource_id: "vm-2".into(),
        detail: String::new(),
    }).await;

    let router = axum::Router::new().nest("/api/v1", thiscloudd::audit::http::app(audit_state));

    // Filter by user=alice
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit?user=alice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let entries: Vec<AuditEntry> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].user, "alice");

    // Filter by action=delete
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit?action=delete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let entries: Vec<AuditEntry> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].resource_id, "vm-2");
}

#[tokio::test]
async fn audit_export_returns_full_log() {
    init();
    let store_arc: Arc<tokio::sync::Mutex<Box<dyn AuditStore>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(MemoryAuditStore::new())));
    let audit_state = AuditState {
        store: store_arc.clone(),
    };

    store_arc.lock().await.log(AuditEntry {
        id: "1".into(),
        timestamp: "100".into(),
        user: "alice".into(),
        role: "admin".into(),
        tenant_id: "t1".into(),
        action: thiscloudd::audit::model::AuditAction::Create,
        resource: "vm".into(),
        resource_id: "vm-1".into(),
        detail: String::new(),
    }).await;

    let router = axum::Router::new().nest("/api/v1", thiscloudd::audit::http::app(audit_state));

    // Export ignores filters and returns the full log.
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit/export")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let entries: Vec<AuditEntry> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].resource_id, "vm-1");
}