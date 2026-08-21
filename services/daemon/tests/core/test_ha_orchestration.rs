// T1.4 — HA: explicit live migration + automatic failover on node loss.
//
// Quorum rule under test: failover requires `online >= max(ha_quorum,
// registered/2 + 1)`. With exactly two nodes a single survivor therefore
// cannot unilaterally move VMs (no split-brain); three nodes can take one
// loss. Explicit migration is always authorized for healthy destinations.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use std::time::Duration;
use thiscloudd::compute::http::{app, ApiState};
use thiscloudd::compute::vm::{VmConfig, VmStatus};
use thiscloudd::compute::{ComputeModule, HypervisorBackend, MemoryVmStore, MockHypervisor};
use thiscloudd::node::model::{Node, NodeRole, NodeState};
use thiscloudd::node::NodeModule;
use tokio::sync::Mutex;
use tower::ServiceExt;

fn node(id: &str, ttl: u64) -> Node {
    Node {
        id: id.to_string(),
        name: format!("node-{}", id),
        role: NodeRole::Worker,
        address: format!("10.0.0.{id}:8080"),
        hostname: format!("node-{id}.local"),
        cpus_total: 8,
        cpus_used: 0,
        memory_total_mb: 16384,
        memory_used_mb: 0,
        vms: 0,
        state: NodeState::Online,
        last_seen_secs: 0,
        ttl_secs: ttl,
        labels: Vec::new(),
    }
}

async fn seed_nodes(nodes: &mut NodeModule, ids: &[&str], ttl: u64) {
    for id in ids {
        nodes.register(node(id, ttl)).await.unwrap();
    }
    // Heartbeat once so every node starts Online before any TTL elapses.
    for id in ids {
        nodes
            .heartbeat(
                id,
                thiscloudd::node::model::NodeHeartbeat {
                    cpus_used: 0,
                    memory_used_mb: 0,
                    vms: 0,
                },
            )
            .await
            .unwrap();
    }
}

fn ha_vm(id: &str, node: &str) -> VmConfig {
    let mut vm = VmConfig::new(
        id.to_string(),
        format!("vm-{}", id),
        2,
        2048,
        format!("/var/lib/thiscloud/vms/{}.qcow2", id),
        vec!["br0".to_string()],
    );
    vm.ha = true;
    vm.node = node.to_string();
    vm
}

fn built_compute(
    hypervisor: Box<dyn HypervisorBackend>,
    nodes: Arc<Mutex<NodeModule>>,
) -> ComputeModule {
    ComputeModule::new(hypervisor, Box::new(MemoryVmStore::default()))
        .with_nodes(nodes)
        .with_ha(true, 2)
}

// ── Explicit live migration ──────────────────────────────────────

#[tokio::test]
async fn test_migrate_running_vm_moves_node_keeps_identity() {
    let nodes = Arc::new(Mutex::new(NodeModule::with_memory_store()));
    seed_nodes(&mut *nodes.lock().await, &["1", "2"], 30).await;
    let mut module = built_compute(Box::new(MockHypervisor::new()), nodes.clone());

    let vm = ha_vm("ha-1", "1");
    module.create_vm("t1", vm.clone()).await.unwrap();
    module.start_vm("t1", "ha-1").await.unwrap();

    let migrated = module.migrate_vm("t1", "ha-1", "2").await.unwrap();

    // Identity preserved: same id, name, disk, network, IP-facing fields.
    assert_eq!(migrated.node, "2");
    assert_eq!(migrated.migrations, 1);
    assert_eq!(migrated.id, vm.id);
    assert_eq!(migrated.name, vm.name);
    assert_eq!(migrated.disk_path, vm.disk_path);
    assert_eq!(migrated.networks, vm.networks);
    assert_eq!(migrated.status, VmStatus::Running);

    // Capacity moved: source released, destination reserved.
    let n1 = nodes.lock().await.get("1").await.unwrap().unwrap();
    let n2 = nodes.lock().await.get("2").await.unwrap().unwrap();
    assert_eq!(n1.cpus_used, 0);
    assert_eq!(n1.vms, 0);
    assert_eq!(n2.cpus_used, 2);
    assert_eq!(n2.vms, 1);
}

#[tokio::test]
async fn test_migrate_to_same_node_or_missing_target_errors() {
    let nodes = Arc::new(Mutex::new(NodeModule::with_memory_store()));
    seed_nodes(&mut *nodes.lock().await, &["1", "2"], 30).await;
    let mut module = built_compute(Box::new(MockHypervisor::new()), nodes);

    module.create_vm("t1", ha_vm("ha-2", "1")).await.unwrap();

    assert!(module.migrate_vm("t1", "ha-2", "1").await.is_err());
    assert!(module.migrate_vm("t1", "ha-2", "").await.is_err());
    assert!(module.migrate_vm("t1", "ha-2", "does-not-exist").await.is_err());
}

#[tokio::test]
async fn test_migrate_stopped_vm_is_placements_only() {
    let nodes = Arc::new(Mutex::new(NodeModule::with_memory_store()));
    seed_nodes(&mut *nodes.lock().await, &["1", "2"], 30).await;
    let mut module = built_compute(Box::new(MockHypervisor::new()), nodes.clone());

    module.create_vm("t1", ha_vm("ha-3", "1")).await.unwrap();
    let migrated = module.migrate_vm("t1", "ha-3", "2").await.unwrap();

    assert_eq!(migrated.node, "2");
    assert_eq!(migrated.status, VmStatus::Stopped);
    assert_eq!(migrated.migrations, 1);
}

/// Destroy node 1 (VM already placed there) by removing it from the registry —
/// deterministic "node lost" simulation, equivalent to an expired heartbeat
/// (which is covered separately below).
async fn drop_node_one(nodes: &Arc<Mutex<NodeModule>>) {
    nodes.lock().await.delete("1").await.unwrap();
}

// ── Automatic failover ───────────────────────────────────────────

#[tokio::test]
async fn test_failover_moves_ha_vm_off_dead_node_with_quorum() {
    let nodes = Arc::new(Mutex::new(NodeModule::with_memory_store()));
    {
        let mut n = nodes.lock().await;
        seed_nodes(&mut n, &["1", "2", "3"], 30).await;
    }
    let mut module = built_compute(Box::new(MockHypervisor::new()), nodes.clone());

    module.create_vm("t1", ha_vm("ha-4", "1")).await.unwrap();
    module.start_vm("t1", "ha-4").await.unwrap();
    drop_node_one(&nodes).await;

    assert_eq!(
        nodes.lock().await.node_state("1").await.unwrap(),
        None // node gone from the registry
    );

    let moved = module.failover_scan().await.unwrap();
    assert_eq!(moved, vec!["ha-4".to_string()]);

    let vm = module.get_vm("t1", "ha-4").await.unwrap();
    assert_eq!(vm.node, "2");
    assert_eq!(vm.migrations, 1);
    assert_eq!(vm.status, VmStatus::Running);
    assert_eq!(vm.networks, vec!["br0".to_string()]);

    // The dead node no longer accounts for the VM's capacity.
    let n1 = nodes.lock().await.get("1").await.unwrap();
    assert!(n1.is_none());
    let n2 = nodes.lock().await.get("2").await.unwrap().unwrap();
    assert_eq!(n2.vms, 1);
    assert_eq!(n2.cpus_used, 2);
}

#[tokio::test]
async fn test_failover_blocked_without_quorum_two_nodes() {
    let nodes = Arc::new(Mutex::new(NodeModule::with_memory_store()));
    {
        let mut n = nodes.lock().await;
        seed_nodes(&mut n, &["1", "2"], 30).await;
    }
    let mut module = built_compute(Box::new(MockHypervisor::new()), nodes.clone());

    module.create_vm("t1", ha_vm("ha-5", "1")).await.unwrap();
    module.start_vm("t1", "ha-5").await.unwrap();
    // Node-1 dies; only node-2 stays up. online(1) < quorum(2) → stay put.
    drop_node_one(&nodes).await;

    let moved = module.failover_scan().await.unwrap();
    assert!(moved.is_empty());

    let vm = module.get_vm("t1", "ha-5").await.unwrap();
    assert_eq!(vm.node, "1");
    assert_eq!(vm.migrations, 0);
}

#[tokio::test]
async fn test_failover_skips_non_ha_vms() {
    let nodes = Arc::new(Mutex::new(NodeModule::with_memory_store()));
    {
        let mut n = nodes.lock().await;
        seed_nodes(&mut n, &["1", "2", "3"], 30).await;
    }
    let mut module = built_compute(Box::new(MockHypervisor::new()), nodes.clone());

    let mut vm = ha_vm("ha-6", "1");
    vm.ha = false; // not enrolled in failover
    module.create_vm("t1", vm).await.unwrap();
    module.start_vm("t1", "ha-6").await.unwrap();
    drop_node_one(&nodes).await;

    let moved = module.failover_scan().await.unwrap();
    assert!(moved.is_empty());
    let vm = module.get_vm("t1", "ha-6").await.unwrap();
    assert_eq!(vm.node, "1");
}

#[tokio::test]
async fn test_failover_disabled_when_ha_off() {
    let nodes = Arc::new(Mutex::new(NodeModule::with_memory_store()));
    {
        let mut n = nodes.lock().await;
        seed_nodes(&mut n, &["1", "2", "3"], 30).await;
    }
    let mut module = ComputeModule::new(Box::new(MockHypervisor::new()), Box::new(MemoryVmStore::default()))
        .with_nodes(nodes.clone())
        .with_ha(false, 2);

    module.create_vm("t1", ha_vm("ha-7", "1")).await.unwrap();
    module.start_vm("t1", "ha-7").await.unwrap();
    drop_node_one(&nodes).await;

    let moved = module.failover_scan().await.unwrap();
    assert!(moved.is_empty());
}

#[tokio::test]
async fn test_failover_detects_ttl_expiry_by_heartbeat() {
    // A node that merely stops heartbeating (still in the registry) is detected
    // via its TTL and its HA VMs are moved off it.
    let nodes = Arc::new(Mutex::new(NodeModule::with_memory_store()));
    {
        let mut n = nodes.lock().await;
        // Node 1 has a 1s TTL (will expire); survivors keep a long TTL so the
        // cluster keeps quorum while only node 1 falls offline.
        n.register(node("1", 1)).await.unwrap();
        n.heartbeat("1", thiscloudd::node::model::NodeHeartbeat { cpus_used: 0, memory_used_mb: 0, vms: 0 }).await.unwrap();
        seed_nodes(&mut n, &["2", "3"], 30).await;
    }
    let mut module = built_compute(Box::new(MockHypervisor::new()), nodes.clone());

    module.create_vm("t1", ha_vm("ha-8", "1")).await.unwrap();
    module.start_vm("t1", "ha-8").await.unwrap();

    // TTL=1s with second-granularity timestamps: wait > 2s to be safe.
    tokio::time::sleep(Duration::from_millis(2200)).await;

    assert_eq!(
        nodes.lock().await.node_state("1").await.unwrap(),
        Some(NodeState::Offline)
    );
    assert_eq!(nodes.lock().await.online_count().await.unwrap(), 2);

    let moved = module.failover_scan().await.unwrap();
    assert_eq!(moved, vec!["ha-8".to_string()]);
    let vm = module.get_vm("t1", "ha-8").await.unwrap();
    assert_eq!(vm.node, "2");
    assert_eq!(vm.migrations, 1);
}

// ── HTTP surface ─────────────────────────────────────────────────

#[tokio::test]
async fn test_http_migrate_vm() {
    let module = ComputeModule::new(
        Box::new(MockHypervisor::new()),
        Box::new(MemoryVmStore::default()),
    );
    let state = ApiState::new(Arc::new(Mutex::new(module)));
    let app = axum::Router::new().nest("/api/v1", app(state));

    // Create a stopped VM, then migrate it (placement-only path).
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "id": "http-ha",
                        "name": "http-ha",
                        "cpus": 2,
                        "memory_mb": 2048,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/http-ha/migrate")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"target_node":"node-9"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["node"], "node-9");
    assert_eq!(body["migrations"], 1);

    // Re-migrating to the same node is an error.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vms/http-ha/migrate")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"target_node":"node-9"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!resp.status().is_success());
}