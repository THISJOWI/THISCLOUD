use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiscloudd::compute::http::{app, ApiState};
use thiscloudd::compute::vm::VmConfig;
use thiscloudd::compute::{ComputeModule, MemoryVmStore, MockHypervisor};
use thiscloudd::metrics::MetricRegistry;
use tokio::sync::Mutex;
use tower::ServiceExt;

fn build_state() -> ApiState {
    let module = ComputeModule::new(
        Box::new(MockHypervisor::new()),
        Box::new(MemoryVmStore::default()),
    );
    ApiState::new(Arc::new(Mutex::new(module)))
}

fn new_app() -> axum::Router {
    axum::Router::new().nest("/api/v1", app(build_state()))
}

async fn seed_vm(app: &axum::Router, id: &str, name: &str) {
    let body = serde_json::to_string(&VmConfig::new(
        id.to_string(),
        name.to_string(),
        2,
        2048,
        format!("/var/lib/thiscloud/vms/{}.qcow2", name),
        vec!["br0".to_string()],
    ))
    .unwrap();
    let resp = app
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
    assert_eq!(resp.status(), StatusCode::CREATED);
}

async fn json_body(app: axum::Router, req: Request<Body>) -> serde_json::Value {
    let resp = app.oneshot(req).await.unwrap();
    assert!(resp.status().is_success(), "status={:?}", resp.status());
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_vm_snapshot_then_restore() {
    let app = new_app();
    seed_vm(&app, "lc-1", "lc1").await;

    let snap = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/lc-1/snapshot")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"golden"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(snap["name"], "golden");
    let snap_id = snap["id"].as_str().unwrap().to_string();

    let vm = json_body(
        app.clone(),
        Request::builder()
            .uri("/api/v1/vms/lc-1")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(vm["snapshots"].as_array().unwrap().len(), 1);
    assert_eq!(vm["snapshots"][0]["id"], snap_id);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/lc-1/restore")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"snapshot_id":"{}"}}"#, snap_id)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_vm_clone() {
    let app = new_app();
    seed_vm(&app, "lc-2", "lc2").await;

    let cloned = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/lc-2/clone")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"lc2-clone"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(cloned["name"], "lc2-clone");
    assert_eq!(cloned["status"], "stopped");
    assert_ne!(cloned["id"], "lc-2");

    let vms = json_body(
        app.clone(),
        Request::builder()
            .uri("/api/v1/vms")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(vms.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_vm_resize_stopped_updates_config() {
    let app = new_app();
    seed_vm(&app, "lc-3", "lc3").await;

    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/lc-3/resize")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"cpus":8,"memory_mb":8192}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["cpus"], 8);
    assert_eq!(vm["memory_mb"], 8192);
}

#[tokio::test]
async fn test_vm_attach_detach_disk() {
    let app = new_app();
    seed_vm(&app, "lc-4", "lc4").await;

    let disk = json_body(
        app.clone(),
        Request::builder()
            .method("PUT")
            .uri("/api/v1/vms/lc-4/disks")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"path":"/var/lib/thiscloud/vms/lc4-data.qcow2","size_gb":50}"#))
            .unwrap(),
    )
    .await;
    let disk_id = disk["id"].as_str().unwrap().to_string();
    assert!(!disk_id.is_empty());

    let vm = json_body(
        app.clone(),
        Request::builder()
            .uri("/api/v1/vms/lc-4")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(vm["disks"].as_array().unwrap().len(), 1);
    assert_eq!(vm["disks"][0]["size_gb"], 50);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/vms/lc-4/disks/{}", disk_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let vm = json_body(
        app,
        Request::builder()
            .uri("/api/v1/vms/lc-4")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(vm["disks"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_vm_attach_detach_nic() {
    let app = new_app();
    seed_vm(&app, "lc-5", "lc5").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/vms/lc-5/nics")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"tap":"br1"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let vm = json_body(
        app.clone(),
        Request::builder()
            .uri("/api/v1/vms/lc-5")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(vm["networks"].as_array().unwrap().contains(&serde_json::json!("br1")));

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/vms/lc-5/nics/br1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let vm = json_body(
        app,
        Request::builder()
            .uri("/api/v1/vms/lc-5")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(!vm["networks"].as_array().unwrap().contains(&serde_json::json!("br1")));
}

#[tokio::test]
async fn test_vm_console_url() {
    let app = new_app();
    seed_vm(&app, "lc-6", "lc6").await;

    let info = json_body(
        app,
        Request::builder()
            .uri("/api/v1/vms/lc-6/console")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let url = info["url"].as_str().unwrap();
    assert!(url.contains("/api/v1/vms/lc-6/console/ws"), "url={}", url);
}

#[tokio::test]
async fn test_vm_console_ws_banner() {
    use futures_util::StreamExt;

    let app = new_app();
    seed_vm(&app, "lc-6", "lc6").await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let url = format!("ws://{addr}/api/v1/vms/lc-6/console/ws");
    let (mut socket, _resp) = tokio_tungstenite::connect_async(url).await.unwrap();
    let msg = socket.next().await.unwrap().unwrap();
    let text = msg.into_text().unwrap();
    assert!(text.contains("THISCLOUD console"), "banner={text:?}");
    assert!(text.contains("lc-6"), "banner={text:?}");
    socket.close(None).await.unwrap();
}

#[tokio::test]
async fn test_vm_create_with_lifecycle_flags() {
    let app = new_app();
    let body = serde_json::json!({
        "name": "secured",
        "cpus": 4,
        "memory_mb": 4096,
        "disk_path": "/var/lib/thiscloud/vms/secured.qcow2",
        "networks": ["br0"],
        "uefi": true,
        "tpm": true,
        "cloud_init": "#cloud-config\nhostname: secured\n",
    });

    let vm = json_body(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["uefi"], true);
    assert_eq!(vm["tpm"], true);
    assert_eq!(
        vm["cloud_init"].as_str().unwrap(),
        "#cloud-config\nhostname: secured\n"
    );
}

// ── T1.6: unified device hotplug ───────────────────────────

#[tokio::test]
async fn test_vm_hotplug_disk_nic_cpu() {
    let app = new_app();
    seed_vm(&app, "vm-hotplug", "hotplug-test").await;

    // Running state exercises the hot paths through the (mock) backend.
    let started = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-hotplug/start")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(started["status"], "running");

    // Hotplug a new blank 10G disk (created then attached).
    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-hotplug/hotplug")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"action":"add","resource":"disk","size_gb":10}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    let disks = vm["disks"].as_array().unwrap();
    assert_eq!(disks.len(), 1);
    assert_eq!(disks[0]["size_gb"], 10);
    let disk_id = disks[0]["id"].as_str().unwrap().to_string();
    assert!(!disks[0]["path"].as_str().unwrap().is_empty());

    // Attach a NIC.
    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-hotplug/hotplug")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"action":"add","resource":"nic","tap":"tap1"}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["networks"].as_array().unwrap().len(), 2); // br0 + tap1

    // Hotplug CPUs (add = resize to 4).
    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-hotplug/hotplug")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"action":"add","resource":"cpu","cpus":4}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["cpus"], 4);

    // Remove the disk and NIC again.
    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-hotplug/hotplug")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"action":"remove","resource":"disk","id":disk_id}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["disks"].as_array().unwrap().len(), 0);

    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-hotplug/hotplug")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"action":"remove","resource":"nic","tap":"tap1"}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["networks"].as_array().unwrap().len(), 1); // back to br0 only
}

#[tokio::test]
async fn test_vm_hotplug_validates_request() {
    let app = new_app();
    seed_vm(&app, "vm-hotplug-err", "hotplug-err").await;

    // Missing path AND size_gb for a disk add.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/vm-hotplug-err/hotplug")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"add","resource":"disk"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Remove disk without an id.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/vm-hotplug-err/hotplug")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"remove","resource":"disk"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Nic add without a tap.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/vm-hotplug-err/hotplug")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action":"add","resource":"nic"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── T1.7: memory ballooning ────────────────────────────────

#[tokio::test]
async fn test_vm_balloon_resize_memory() {
    let app = new_app();

    // Create a VM with balloon bounds 1024..4096 MB.
    let body = serde_json::json!({
        "id": "vm-balloon",
        "name": "balloon-test",
        "cpus": 2,
        "memory_mb": 2048,
        "disk_path": "/var/lib/thiscloud/vms/balloon-test.qcow2",
        "balloon": { "min_mb": 1024, "max_mb": 4096 },
    });
    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["balloon"]["min_mb"], 1024);
    assert_eq!(vm["balloon"]["max_mb"], 4096);
    assert_eq!(vm["memory_mb"], 2048);

    // Start it (running state exercises the balloon hot path).
    let started = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-balloon/start")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(started["status"], "running");

    // Grow to 3072 MB — inside bounds.
    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-balloon/memory")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"target_mb": 3072}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["memory_mb"], 3072);

    // Shrink to 1536 MB — inside bounds.
    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-balloon/memory")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"target_mb": 1536}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["memory_mb"], 1536);

    // Below min: rejected with 400.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/vm-balloon/memory")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"target_mb": 512}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Above max: rejected with 400.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/vm-balloon/memory")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"target_mb": 8192}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_vm_balloon_requires_config() {
    let app = new_app();
    seed_vm(&app, "vm-no-balloon", "no-balloon").await;

    // A VM without balloon bounds still honours the memory endpoint but must
    // accept any positive target (no min/max to constrain it).
    let vm = json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-no-balloon/memory")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"target_mb": 4096}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(vm["memory_mb"], 4096);

    // Zero target is always rejected.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/vm-no-balloon/memory")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"target_mb": 0}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_vm_balloon_publishes_metric() {
    let registry = Arc::new(MetricRegistry::new());
    let module = ComputeModule::new(
        Box::new(MockHypervisor::new()),
        Box::new(MemoryVmStore::default()),
    )
    .with_metrics(registry.clone());
    let state = ApiState::new(Arc::new(Mutex::new(module)));
    let app = axum::Router::new().nest("/api/v1", app(state));

    let body = serde_json::json!({
        "id": "vm-balloon-metric",
        "name": "balloon-metric",
        "cpus": 1,
        "memory_mb": 2048,
        "disk_path": "/var/lib/thiscloud/vms/balloon-metric.qcow2",
        "balloon": { "min_mb": 1024, "max_mb": 4096 },
    });
    json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await;

    json_body(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/v1/vms/vm-balloon-metric/memory")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"target_mb": 3072}).to_string(),
            ))
            .unwrap(),
    )
    .await;

    let mut labels = BTreeMap::new();
    labels.insert("vm_id".to_string(), "vm-balloon-metric".to_string());
    labels.insert("vm_name".to_string(), "balloon-metric".to_string());
    let snap = registry.snapshot();
    let gauge = snap
        .iter()
        .find(|m| m.name == "thiscloud_vm_memory_mb" && m.labels == labels)
        .expect("memory gauge published");
    assert_eq!(gauge.value, 3072.0);
}
