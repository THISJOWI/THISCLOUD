//! T5.1 HTTP tests: anonymous scrape, RBAC-protected push, push semantics.

use crate::http::{app, MetricsApiState};
use crate::registry::MetricRegistry;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::collections::BTreeMap;
use std::sync::Arc;
use tower::ServiceExt;

fn router() -> axum::Router {
    app(MetricsApiState::new(Arc::new(MetricRegistry::new())))
}

fn auth_ctx(role: thiscloudd::auth::Role) -> thiscloudd::auth::AuthContext {
    thiscloudd::auth::AuthContext {
        claims: thiscloudd::auth::Claims {
            sub: "user-1".into(),
            tenant_id: "tenant-a".into(),
            role,
            exp: 9999999999,
            iat: 1000000000,
        },
    }
}

fn push_request(body: &str, role: Option<thiscloudd::auth::Role>) -> Request<Body> {
    let mut req = Request::builder()
        .method("POST")
        .uri("/metrics/push")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    if let Some(role) = role {
        req.extensions_mut().insert(auth_ctx(role));
    }
    req
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn get_metrics_returns_200_with_type_line() {
    let registry = Arc::new(MetricRegistry::new());
    registry.set_gauge("thiscloud_vms", 3.0, BTreeMap::new());
    let r = app(MetricsApiState::new(registry));
    let resp = r
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("# TYPE thiscloud_vms gauge"));
    assert!(body.contains("thiscloud_vms 3"));
}

#[tokio::test]
async fn get_metrics_works_without_tenant_header() {
    // Anonymous scrape: no x-tenant header, no Authorization header. The route
    // must NOT require TenantContext extraction (which would 4xx without one).
    let registry = Arc::new(MetricRegistry::new());
    registry.set_gauge("thiscloud_vms", 1.0, BTreeMap::new());
    let r = app(MetricsApiState::new(registry));
    let resp = r
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_metrics_content_type_is_prometheus_text() {
    let registry = Arc::new(MetricRegistry::new());
    registry.set_gauge("thiscloud_vms", 1.0, BTreeMap::new());
    let r = app(MetricsApiState::new(registry));
    let resp = r
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        resp.headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap(),
        "text/plain; version=0.0.4"
    );
}

#[tokio::test]
async fn push_rejects_without_write_role() {
    // TenantUser is in CreateSet but NOT WriteSet → 403.
    let r = router();
    let body = r#"{"name":"thiscloud_vms","value":1.0,"metric_type":"gauge"}"#;
    let resp = r.oneshot(push_request(body, Some(thiscloudd::auth::Role::TenantUser))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn push_rejects_auditor() {
    let r = router();
    let body = r#"{"name":"thiscloud_vms","value":1.0,"metric_type":"gauge"}"#;
    let resp = r.oneshot(push_request(body, Some(thiscloudd::auth::Role::Auditor))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn push_with_write_role_returns_200_and_registers_gauge() {
    let registry = Arc::new(MetricRegistry::new());
    let r = app(MetricsApiState::new(registry.clone()));
    let body = r#"{"name":"thiscloud_vms","value":2.0,"metric_type":"gauge"}"#;
    let resp = r
        .oneshot(push_request(body, Some(thiscloudd::auth::Role::Admin)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let snap = registry.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].name, "thiscloud_vms");
    assert_eq!(snap[0].value, 2.0);
}

#[tokio::test]
async fn push_accepts_array() {
    let registry = Arc::new(MetricRegistry::new());
    let r = app(MetricsApiState::new(registry.clone()));
    let body = r#"[
        {"name":"thiscloud_vms","value":1.0,"metric_type":"gauge"},
        {"name":"thiscloud_api_requests_total","value":7.0,"metric_type":"counter"}
    ]"#;
    let resp = r
        .oneshot(push_request(body, Some(thiscloudd::auth::Role::Operator)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(registry.snapshot().len(), 2);
}

#[tokio::test]
async fn push_counter_increments_on_repeat() {
    let registry = Arc::new(MetricRegistry::new());
    let r = app(MetricsApiState::new(registry.clone()));
    let body = r#"{"name":"thiscloud_api_requests_total","value":3.0,"metric_type":"counter"}"#;
    let req = push_request(body, Some(thiscloudd::auth::Role::Admin));
    let resp = r.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let req = push_request(body, Some(thiscloudd::auth::Role::Admin));
    let resp = r.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let snap = registry.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].value, 6.0);
}

#[tokio::test]
async fn push_rejects_empty_name() {
    let r = router();
    let body = r#"{"name":"","value":1.0,"metric_type":"gauge"}"#;
    let resp = r
        .oneshot(push_request(body, Some(thiscloudd::auth::Role::Admin)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn pushed_metric_appears_in_scrape() {
    let registry = Arc::new(MetricRegistry::new());
    let r = app(MetricsApiState::new(registry.clone()));
    let body = r#"{"name":"thiscloud_vms","value":4.0,"metric_type":"gauge"}"#;
    let resp = r
        .clone()
        .oneshot(push_request(body, Some(thiscloudd::auth::Role::Admin)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = r
        .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("# TYPE thiscloud_vms gauge"));
    assert!(body.contains("thiscloud_vms 4"));
}