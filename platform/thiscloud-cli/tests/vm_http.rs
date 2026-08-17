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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_vm_hotplug() {
    let server = ApiServer::start().await;

    Command::new(cli_bin())
        .args(["vm", "create", "--name", "hot1"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    // Resolve the server-assigned id.
    let output = Command::new(cli_bin())
        .args(["vm", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let hot1_id = stdout
        .lines()
        .find(|l| l.contains("hot1"))
        .and_then(|l| l.split_whitespace().next())
        .expect("hot1 not found in list");

    // Start it so hotplug runs against a running VM.
    let output = Command::new(cli_bin())
        .args(["vm", "start", hot1_id])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    // Hotplug a blank 10G disk.
    let output = Command::new(cli_bin())
        .args(["vm", "hotplug", hot1_id, "disk", "--size-gb", "10"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "disk hotplug failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 disks"), "expected 1 disk: {}", stdout);

    // Hotplug a NIC.
    let output = Command::new(cli_bin())
        .args(["vm", "hotplug", hot1_id, "nic", "--tap", "tap9"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    // Hotplug CPUs to 4.
    let output = Command::new(cli_bin())
        .args(["vm", "hotplug", hot1_id, "cpu", "--cpus", "4"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("4 cpus"), "expected 4 cpus: {}", stdout);

    // Invalid resource is rejected.
    let output = Command::new(cli_bin())
        .args(["vm", "hotplug", hot1_id, "gpu", "--cpus", "1"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_vm_resize_memory() {
    let server = ApiServer::start().await;

    let output = Command::new(cli_bin())
        .args([
            "vm", "create", "--name", "mem1", "--cpus", "1", "--memory", "2048",
        ])
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
    let mem1_id = stdout
        .lines()
        .find(|l| l.contains("mem1"))
        .and_then(|l| l.split_whitespace().next())
        .expect("mem1 not found in list");

    // Resize memory via balloon endpoint (no balloon bounds on a plain VM).
    let output = Command::new(cli_bin())
        .args(["vm", "resize-memory", mem1_id, "--target-mb", "4096"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "resize-memory failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("4096 MB"), "expected 4096 MB: {}", stdout);

    // Zero target rejected (validation error).
    let output = Command::new(cli_bin())
        .args(["vm", "resize-memory", mem1_id, "--target-mb", "0"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(!output.status.success());
}
