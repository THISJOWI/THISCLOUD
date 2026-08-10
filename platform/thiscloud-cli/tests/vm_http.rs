use std::process::Command;
use std::sync::Arc;

use thiscloudd::compute::http::{app, ApiState};
use thiscloudd::compute::{ComputeModule, MemoryVmStore, MockHypervisor};

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_thiscloud")
}

struct ApiServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl ApiServer {
    async fn start() -> Self {
        let module = ComputeModule::new(
            Box::new(MockHypervisor::new()),
            Box::new(MemoryVmStore::default()),
        );
        let state = ApiState::new(Arc::new(tokio::sync::Mutex::new(module)));
        let router = app(state);

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
async fn test_cli_vm_create_and_list() {
    let server = ApiServer::start().await;

    let output = Command::new(cli_bin())
        .args([
            "vm", "create", "--name", "web1", "--cpus", "2", "--memory", "2048",
        ])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("web1"));

    let output = Command::new(cli_bin())
        .args(["vm", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("web1"));
    assert!(stdout.contains("2"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_vm_start_and_stop() {
    let server = ApiServer::start().await;

    Command::new(cli_bin())
        .args([
            "vm", "create", "--name", "db1", "--cpus", "4", "--memory", "4096",
        ])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    // List VMs to get the server-assigned UUID.
    let output = Command::new(cli_bin())
        .args(["vm", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let db1_id = stdout
        .lines()
        .find(|l| l.contains("db1"))
        .and_then(|l| l.split_whitespace().next())
        .expect("db1 not found in list");

    let output = Command::new(cli_bin())
        .args(["vm", "start", db1_id])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "start failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(cli_bin())
        .args(["vm", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("running"));

    let output = Command::new(cli_bin())
        .args(["vm", "stop", db1_id])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(cli_bin())
        .args(["vm", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stopped"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_vm_delete() {
    let server = ApiServer::start().await;

    Command::new(cli_bin())
        .args(["vm", "create", "--name", "del1"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    // List VMs to get the server-assigned UUID.
    let output = Command::new(cli_bin())
        .args(["vm", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let del1_id = stdout
        .lines()
        .find(|l| l.contains("del1"))
        .and_then(|l| l.split_whitespace().next())
        .expect("del1 not found in list");

    let output = Command::new(cli_bin())
        .args(["vm", "delete", del1_id])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(cli_bin())
        .args(["vm", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("del1"));
}
