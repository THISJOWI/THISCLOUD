use std::process::Command;
use std::sync::Arc;

use thiscloudd::compute::http::{app as vm_app, ApiState as VmApiState};
use thiscloudd::compute::{ComputeModule, MemoryVmStore, MockHypervisor};
use thiscloudd::node::http::{app as node_app, NodeApiState};
use thiscloudd::node::model::{Node, NodeRole, NodeState};
use thiscloudd::node::{MemoryNodeStore, NodeModule, NodeStore};

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_thiscloud")
}

struct ApiServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl ApiServer {
    async fn start() -> Self {
        // Health endpoint is mounted at /api/v1/healthz by the daemon.
        let health_router = axum::Router::new()
            .route(
                "/api/v1/healthz",
                axum::routing::get(|| async { axum::Json(serde_json::json!({"status":"ok"})) }),
            )
            .route(
                "/api/v1/ready",
                axum::routing::get(|| async {
                    axum::Json(serde_json::json!({"status":"ready","checks":{"etcd":true}}))
                }),
            );

        let compute = ComputeModule::new(
            Box::new(MockHypervisor::new()),
            Box::new(MemoryVmStore::default()),
        );
        let vm_router = vm_app(VmApiState::new(Arc::new(tokio::sync::Mutex::new(compute))));

        let node_store = MemoryNodeStore::default();
        node_store
            .put(&Node {
                id: "master-1".to_string(),
                name: "localhost".to_string(),
                role: NodeRole::Master,
                address: "127.0.0.1:8080".to_string(),
                hostname: "localhost".to_string(),
                cpus_total: 4,
                cpus_used: 0,
                memory_total_mb: 8192,
                memory_used_mb: 0,
                vms: 0,
                state: NodeState::Online,
                last_seen_secs: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                ttl_secs: 30,
                labels: vec![],
            })
            .await
            .unwrap();
        let nodes = NodeModule::new(Box::new(node_store));
        let node_router = node_app(NodeApiState::new(Arc::new(tokio::sync::Mutex::new(nodes))));

        let router = health_router
            .merge(axum::Router::new().nest("/api/v1", vm_router))
            .merge(axum::Router::new().nest("/api/v1", node_router));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        Self {
            base_url: format!("http://{}", addr),
            handle,
        }
    }
}

impl Drop for ApiServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_status_reports_daemon_nodes_vms() {
    let server = ApiServer::start().await;

    let output = Command::new(cli_bin())
        .args(["status"])
        .env("THISCLOUD_API_URL", server.base_url.clone())
        .output()
        .expect("failed to run thiscloud status");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Daemon:   Running"),
        "expected daemon running, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Readiness: Ready"),
        "expected readiness ready, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Nodes:    1 (1 online, 0 offline)"),
        "expected node summary, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("VMs:      0 (0 running, 0 stopped)"),
        "expected vm summary, got:\n{}",
        stdout
    );
    assert!(
        output.status.success(),
        "status command exited non-zero: {}",
        stdout
    );
}