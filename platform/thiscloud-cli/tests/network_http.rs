use std::process::Command;
use std::sync::Arc;

use thiscloudd::network::http::{app, NetworkApiState};
use thiscloudd::network::{MemoryNetworkStore, MockNetworkBackend, NetworkModule};

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_thiscloud")
}

struct ApiServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl ApiServer {
    async fn start() -> Self {
        let module = NetworkModule::new(
            Box::new(MockNetworkBackend::new()),
            Box::new(MemoryNetworkStore::default()),
        );
        let state = NetworkApiState::new(Arc::new(tokio::sync::Mutex::new(module)));
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
async fn test_cli_network_create_and_list() {
    let server = ApiServer::start().await;

    let output = Command::new(cli_bin())
        .args([
            "network",
            "create",
            "--name",
            "web",
            "--cidr",
            "10.0.0.0/24",
            "--gateway",
            "10.0.0.1",
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
    assert!(stdout.contains("web"));

    let output = Command::new(cli_bin())
        .args(["network", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("web"));
    assert!(stdout.contains("10.0.0.0/24"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_network_delete() {
    let server = ApiServer::start().await;

    Command::new(cli_bin())
        .args([
            "network",
            "create",
            "--name",
            "delnet",
            "--cidr",
            "10.1.0.0/24",
        ])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    // List networks to get the server-assigned UUID.
    let output = Command::new(cli_bin())
        .args(["network", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let delnet_id = stdout
        .lines()
        .find(|l| l.contains("delnet"))
        .and_then(|l| l.split_whitespace().next())
        .expect("delnet not found in list");

    let output = Command::new(cli_bin())
        .args(["network", "delete", delnet_id])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(cli_bin())
        .args(["network", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("delnet"));
}
