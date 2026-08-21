use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use thiscloudd::network::http::{app, NetworkApiState};
use thiscloudd::network::{
    FloatingIp, FloatingIpStore, LogicalNetwork, MemoryFloatingIpStore, MemoryNetworkStore,
    MockNetworkBackend, NetworkModule,
};
use tower::ServiceExt;

fn module() -> NetworkModule {
    NetworkModule::new(
        Box::new(MockNetworkBackend::new()),
        Box::new(MemoryNetworkStore::default()),
    )
}

fn make_router() -> axum::Router {
    let module = NetworkModule::new(
        Box::new(MockNetworkBackend::new()),
        Box::new(MemoryNetworkStore::default()),
    );
    axum::Router::new().nest(
        "/api/v1",
        app(NetworkApiState::new(Arc::new(tokio::sync::Mutex::new(
            module,
        )))),
    )
}

/// Create a tenant network the floating IP can reference.
async fn seed_network(m: &mut NetworkModule, tenant: &str, id: &str) {
    let mut net = LogicalNetwork::new(
        id.to_string(),
        id.to_string(),
        "10.0.0.0/24".to_string(),
        "10.0.0.1".to_string(),
    );
    m.create_network(tenant, &mut net).await.unwrap();
}

// --- model ---

#[test]
fn test_floating_model_serde_roundtrip() {
    let fip = FloatingIp::new("f-1".to_string(), "web-ip".to_string(), "10.0.0.5".to_string());
    let json = serde_json::to_string(&fip).unwrap();
    let parsed: FloatingIp = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, fip);
}

#[test]
fn test_floating_model_defaults() {
    let fip = FloatingIp::new("f-1".to_string(), "web-ip".to_string(), "10.0.0.5".to_string());
    assert!(fip.vm_id.is_none());
    assert!(fip.net_id.is_none());
    assert!(fip.tenant_id.is_empty());
}

// --- module: allocation ---

#[tokio::test]
async fn test_floating_allocate_explicit_ip() {
    let mut m = module();
    seed_network(&mut m, "tenant-a", "net-1").await;
    let mut fip = FloatingIp::new(String::new(), "web-ip".to_string(), "10.0.0.50".to_string());
    fip.net_id = Some("net-1".to_string());
    m.allocate_floating_ip("tenant-a", &mut fip, Some("10.0.0.50".to_string()))
        .await
        .unwrap();
    assert!(!fip.id.is_empty());
    assert_eq!(fip.ip, "10.0.0.50");
    assert_eq!(fip.tenant_id, "tenant-a");
    assert_eq!(m.list_floating_ips("tenant-a").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_floating_allocate_from_cidr() {
    let mut m = module();
    seed_network(&mut m, "tenant-a", "net-1").await;
    // No explicit IP: first free host after the gateway (10.0.0.1) is 10.0.0.2.
    let mut fip = FloatingIp::new(String::new(), "auto-ip".to_string(), String::new());
    fip.net_id = Some("net-1".to_string());
    m.allocate_floating_ip("tenant-a", &mut fip, None).await.unwrap();
    assert_eq!(fip.ip, "10.0.0.2");

    // Second allocation skips the already-used 10.0.0.2.
    let mut fip2 = FloatingIp::new(String::new(), "auto-ip-2".to_string(), String::new());
    fip2.net_id = Some("net-1".to_string());
    m.allocate_floating_ip("tenant-a", &mut fip2, None).await.unwrap();
    assert_eq!(fip2.ip, "10.0.0.3");
}

#[tokio::test]
async fn test_floating_allocate_requires_net_for_auto() {
    let mut m = module();
    let mut fip = FloatingIp::new(String::new(), "orphan".to_string(), String::new());
    let err = m.allocate_floating_ip("tenant-a", &mut fip, None).await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("net_id required"));
}

#[tokio::test]
async fn test_floating_duplicate_ip_errors() {
    let mut m = module();
    seed_network(&mut m, "tenant-a", "net-1").await;
    let mut fip = FloatingIp::new(String::new(), "web-ip".to_string(), "10.0.0.50".to_string());
    fip.net_id = Some("net-1".to_string());
    m.allocate_floating_ip("tenant-a", &mut fip, Some("10.0.0.50".to_string()))
        .await
        .unwrap();

    let mut dup = FloatingIp::new(String::new(), "other-ip".to_string(), "10.0.0.50".to_string());
    dup.net_id = Some("net-1".to_string());
    let err = m
        .allocate_floating_ip("tenant-a", &mut dup, Some("10.0.0.50".to_string()))
        .await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("already allocated"));
}

#[tokio::test]
async fn test_floating_duplicate_name_errors() {
    let mut m = module();
    seed_network(&mut m, "tenant-a", "net-1").await;
    let mut fip = FloatingIp::new(String::new(), "web-ip".to_string(), "10.0.0.50".to_string());
    fip.net_id = Some("net-1".to_string());
    m.allocate_floating_ip("tenant-a", &mut fip, Some("10.0.0.50".to_string()))
        .await
        .unwrap();

    let mut dup = FloatingIp::new(String::new(), "web-ip".to_string(), "10.0.0.51".to_string());
    dup.net_id = Some("net-1".to_string());
    let err = m
        .allocate_floating_ip("tenant-a", &mut dup, Some("10.0.0.51".to_string()))
        .await;
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("already exists"));
}

#[tokio::test]
async fn test_floating_deallocate() {
    let mut m = module();
    seed_network(&mut m, "tenant-a", "net-1").await;
    let mut fip = FloatingIp::new(String::new(), "web-ip".to_string(), "10.0.0.50".to_string());
    fip.net_id = Some("net-1".to_string());
    m.allocate_floating_ip("tenant-a", &mut fip, Some("10.0.0.50".to_string()))
        .await
        .unwrap();
    m.deallocate_floating_ip("tenant-a", &fip.id).await.unwrap();
    assert!(m.list_floating_ips("tenant-a").await.unwrap().is_empty());
}

#[tokio::test]
async fn test_floating_tenant_isolation() {
    let mut m = module();
    seed_network(&mut m, "tenant-a", "net-1").await;
    seed_network(&mut m, "tenant-b", "net-1").await;

    let mut fip = FloatingIp::new(String::new(), "web-ip".to_string(), "10.0.0.50".to_string());
    fip.net_id = Some("net-1".to_string());
    m.allocate_floating_ip("tenant-a", &mut fip, Some("10.0.0.50".to_string()))
        .await
        .unwrap();

    // Same IP is fine in tenant-b; tenant-a's allocation is invisible there.
    let mut fip_b = FloatingIp::new(String::new(), "web-ip".to_string(), "10.0.0.50".to_string());
    fip_b.net_id = Some("net-1".to_string());
    m.allocate_floating_ip("tenant-b", &mut fip_b, Some("10.0.0.50".to_string()))
        .await
        .unwrap();
    assert_eq!(m.list_floating_ips("tenant-a").await.unwrap().len(), 1);
    assert_eq!(m.list_floating_ips("tenant-b").await.unwrap().len(), 1);
}

// --- HTTP ---

#[tokio::test]
async fn test_floating_http_allocate_and_list() {
    let router = make_router();

    // Seed a network via the API so auto-allocation has a CIDR to draw from.
    let net_body = r#"{
        "id": "net-1",
        "name": "web",
        "cidr": "10.0.0.0/24",
        "gateway": "10.0.0.1"
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/networks")
                .header("content-type", "application/json")
                .body(Body::from(net_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let fip_body = r#"{
        "name": "web-ip",
        "net_id": "net-1"
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/network/floating-ips")
                .header("content-type", "application/json")
                .body(Body::from(fip_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/network/floating-ips")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_floating_http_get_missing_404() {
    let router = make_router();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/network/floating-ips/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// --- store sanity (memory) ---

#[tokio::test]
async fn test_floating_store_roundtrip() {
    let store = MemoryFloatingIpStore::default();
    let fip = FloatingIp::new("f-1".to_string(), "web-ip".to_string(), "10.0.0.5".to_string());
    store.put("tenant-a", &fip).await.unwrap();
    let got = store.get("tenant-a", "f-1").await.unwrap().unwrap();
    assert_eq!(got.ip, "10.0.0.5");
    store.delete("tenant-a", "f-1").await.unwrap();
    assert!(store.get("tenant-a", "f-1").await.unwrap().is_none());
}