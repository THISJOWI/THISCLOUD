use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use thiscloudd::network::http::{app, NetworkApiState};
use thiscloudd::network::{
    DhcpServer, DhcpStore, MemoryDhcpStore, MemoryNetworkStore, MemoryRouterStore,
    MockNetworkBackend, NetworkModule, NetworkStatus, RouterStore, VirtualRouter,
};
use tower::ServiceExt;

fn sample_router(id: &str) -> VirtualRouter {
    VirtualRouter::new(id.to_string(), id.to_string())
}

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

// --- model ---

#[test]
fn test_router_model_serde_roundtrip() {
    let router = VirtualRouter::new("r-1".to_string(), "edge".to_string());
    let json = serde_json::to_string(&router).unwrap();
    let parsed: VirtualRouter = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, router);
}

#[test]
fn test_router_model_defaults() {
    let router = VirtualRouter::new("r-1".to_string(), "edge".to_string());
    assert_eq!(router.status, NetworkStatus::Created);
    assert!(!router.ha);
    assert_eq!(router.net_id, None);
    assert_eq!(router.external_net_id, None);
    assert!(router.tenant_id.is_empty());
}

#[test]
fn test_dhcp_model_serde_roundtrip() {
    let dhcp = DhcpServer::new(
        "d-1".to_string(),
        "lan-dhcp".to_string(),
        "net-1".to_string(),
        "10.0.0.10".to_string(),
        "10.0.0.200".to_string(),
    );
    let json = serde_json::to_string(&dhcp).unwrap();
    let parsed: DhcpServer = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, dhcp);
}

#[test]
fn test_dhcp_model_defaults() {
    let dhcp = DhcpServer::new(
        "d-1".to_string(),
        "lan-dhcp".to_string(),
        "net-1".to_string(),
        "10.0.0.10".to_string(),
        "10.0.0.200".to_string(),
    );
    assert_eq!(dhcp.status, NetworkStatus::Created);
    assert!(dhcp.dns.is_empty());
    assert!(dhcp.tenant_id.is_empty());
}

// --- module: routers ---

#[tokio::test]
async fn test_router_module_create_and_list() {
    let mut m = module();
    m.create_router("", &mut sample_router("edge")).await.unwrap();
    m.create_router("", &mut sample_router("core")).await.unwrap();
    assert_eq!(m.list_routers("").await.unwrap().len(), 2);
}

#[tokio::test]
async fn test_router_module_get() {
    let mut m = module();
    m.create_router("", &mut sample_router("edge")).await.unwrap();
    let router = m.get_router("", "edge").await.unwrap();
    assert_eq!(router.name, "edge");
    assert_eq!(router.tenant_id, "");
}

#[tokio::test]
async fn test_router_module_duplicate_name_errors() {
    let mut m = module();
    m.create_router("", &mut sample_router("edge")).await.unwrap();
    let mut dup = sample_router("edge");
    let err = m.create_router("", &mut dup).await.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[tokio::test]
async fn test_router_module_delete() {
    let mut m = module();
    m.create_router("", &mut sample_router("edge")).await.unwrap();
    m.delete_router("", "edge").await.unwrap();
    assert!(m.list_routers("").await.unwrap().is_empty());
}

#[tokio::test]
async fn test_router_module_tenant_isolation() {
    let mut m = module();
    m.create_router("tenant-a", &mut sample_router("edge"))
        .await
        .unwrap();
    assert!(m.list_routers("tenant-b").await.unwrap().is_empty());
    assert!(m.get_router("tenant-b", "edge").await.is_err());
    // Same name is fine in a different tenant.
    m.create_router("tenant-b", &mut sample_router("edge"))
        .await
        .unwrap();
    assert_eq!(m.list_routers("tenant-b").await.unwrap().len(), 1);
}

// --- module: dhcp ---

#[tokio::test]
async fn test_dhcp_module_create_get_delete() {
    let mut m = module();
    let mut dhcp = DhcpServer::new(
        String::new(),
        "lan-dhcp".to_string(),
        "net-1".to_string(),
        "10.0.0.10".to_string(),
        "10.0.0.200".to_string(),
    );
    m.create_dhcp("tenant-a", &mut dhcp).await.unwrap();
    assert!(!dhcp.id.is_empty());
    assert_eq!(dhcp.tenant_id, "tenant-a");
    assert_eq!(m.list_dhcp("tenant-a").await.unwrap().len(), 1);
    let got = m.get_dhcp("tenant-a", &dhcp.id).await.unwrap();
    assert_eq!(got.name, "lan-dhcp");
    m.delete_dhcp("tenant-a", &dhcp.id).await.unwrap();
    assert!(m.list_dhcp("tenant-a").await.unwrap().is_empty());
}

#[tokio::test]
async fn test_dhcp_module_duplicate_name_errors() {
    let mut m = module();
    let mut dhcp = DhcpServer::new(
        String::new(),
        "lan-dhcp".to_string(),
        "net-1".to_string(),
        "10.0.0.10".to_string(),
        "10.0.0.200".to_string(),
    );
    m.create_dhcp("", &mut dhcp).await.unwrap();
    let mut dup = DhcpServer::new(
        String::new(),
        "lan-dhcp".to_string(),
        "net-2".to_string(),
        "10.0.1.10".to_string(),
        "10.0.1.200".to_string(),
    );
    let err = m.create_dhcp("", &mut dup).await.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

// --- HTTP ---

#[tokio::test]
async fn test_router_http_create_and_list() {
    let router = make_router();

    let create_body = r#"{
        "id": "r-1",
        "name": "edge",
        "ha": true
    }"#;
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/network/routers")
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
                .uri("/api/v1/network/routers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_router_http_get_missing_404() {
    let router = make_router();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/network/routers/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// --- store sanity (memory) ---

#[tokio::test]
async fn test_router_store_composite_key_isolates_tenants() {
    let store = MemoryRouterStore::default();
    let mut a = sample_router("edge");
    a.tenant_id = "tenant-a".into();
    let mut b = sample_router("edge");
    b.tenant_id = "tenant-b".into();
    store.put("tenant-a", &a).await.unwrap();
    store.put("tenant-b", &b).await.unwrap();
    assert_eq!(store.list("tenant-a").await.unwrap().len(), 1);
    assert_eq!(store.list("tenant-b").await.unwrap().len(), 1);
    assert_eq!(store.list("").await.unwrap().len(), 2);
}

#[tokio::test]
async fn test_dhcp_store_roundtrip() {
    let store = MemoryDhcpStore::default();
    let dhcp = DhcpServer::new(
        "d-1".to_string(),
        "lan-dhcp".to_string(),
        "net-1".to_string(),
        "10.0.0.10".to_string(),
        "10.0.0.200".to_string(),
    );
    store.put("tenant-a", &dhcp).await.unwrap();
    let got = store.get("tenant-a", "d-1").await.unwrap().unwrap();
    assert_eq!(got.name, "lan-dhcp");
    store.delete("tenant-a", "d-1").await.unwrap();
    assert!(store.get("tenant-a", "d-1").await.unwrap().is_none());
}