//! T0.5 Quota tests: set/get/list/delete tenant quotas, and quota enforcement.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use thiscloudd::quota::model::TenantQuota;
use thiscloudd::quota::module::QuotaModule;
use thiscloudd::quota::http::{app, QuotaApiState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

fn router() -> axum::Router {
    let module = QuotaModule::with_memory_store();
    axum::Router::new().nest("/api/v1", app(QuotaApiState::new(Arc::new(Mutex::new(module)))))
}

#[tokio::test]
async fn set_and_get_quota() {
    let r = router();
    let body = r#"{"tenant_id":"t1","max_cpus":4,"max_memory_mb":8192,"max_vms":2,"max_storage_gb":100,"max_networks":3}"#;
    let resp = r
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/quotas/t1")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/quotas/t1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let q: TenantQuota = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(q.max_cpus, 4);
    assert_eq!(q.max_vms, 2);
}

#[tokio::test]
async fn list_quotas_empty() {
    let r = router();
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/quotas")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let list: Vec<TenantQuota> = serde_json::from_slice(&bytes).unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn list_quotas_with_entries() {
    let r = router();
    let body = r#"{"tenant_id":"t1","max_cpus":4,"max_memory_mb":1,"max_vms":1,"max_storage_gb":1,"max_networks":1}"#;
    r.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/quotas/t1")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/quotas")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let list: Vec<TenantQuota> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].tenant_id, "t1");
}

#[tokio::test]
async fn delete_quota_returns_204() {
    let r = router();
    let body = r#"{"tenant_id":"t2","max_cpus":1,"max_memory_mb":1,"max_vms":1,"max_storage_gb":1,"max_networks":1}"#;
    r.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/quotas/t2")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let resp = r
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/quotas/t2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn get_quota_returns_unlimited_by_default() {
    let r = router();
    let resp = r
        .oneshot(
            Request::builder()
                .uri("/api/v1/quotas/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let q: TenantQuota = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(q.max_cpus, 0); // 0 = unlimited
    assert_eq!(q.max_vms, 0);
}

#[tokio::test]
async fn quota_enforcement_check() {
    let module = QuotaModule::with_memory_store();
    // No quota set → all unlimited
    assert!(module
        .check("t1", &thiscloudd::quota::model::ResourceDelta::default())
        .await
        .is_ok());

    // Set limits
    let mut q = TenantQuota::unlimited("t1");
    q.max_vms = 2;
    let m = module;
    m.set(q).await.unwrap();

    assert!(m
        .check(
            "t1",
            &thiscloudd::quota::model::ResourceDelta {
                vms: 2,
                ..Default::default()
            }
        )
        .await
        .is_ok());

    assert!(m
        .check(
            "t1",
            &thiscloudd::quota::model::ResourceDelta {
                vms: 3,
                ..Default::default()
            }
        )
        .await
        .is_err());
}