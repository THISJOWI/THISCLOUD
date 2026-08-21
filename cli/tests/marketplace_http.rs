use std::process::Command;
use std::sync::Arc;

use thiscloudd::marketplace::http::{app, MarketplaceApiState};
use thiscloudd::marketplace::{MarketplaceModule, MemoryMarketplaceStore, MockMarketplaceBackend};

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_thiscloud")
}

struct ApiServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl ApiServer {
    async fn start() -> Self {
        let module = MarketplaceModule::new(
            Box::new(MockMarketplaceBackend::default()),
            Box::new(MemoryMarketplaceStore::default()),
        );
        let state = MarketplaceApiState::new(Arc::new(tokio::sync::Mutex::new(module)));
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
async fn test_cli_marketplace_install_and_list() {
    let server = ApiServer::start().await;

    let output = Command::new(cli_bin())
        .args([
            "marketplace",
            "install",
            "--name",
            "nginx",
            "--app-type",
            "docker",
            "--source",
            "nginx:latest",
        ])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nginx"));

    let output = Command::new(cli_bin())
        .args(["marketplace", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nginx"));
    assert!(stdout.contains("docker"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_marketplace_uninstall() {
    let server = ApiServer::start().await;

    let output = Command::new(cli_bin())
        .args([
            "marketplace",
            "install",
            "--name",
            "redis",
            "--source",
            "redis:7",
        ])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    // Grab the id from the list output.
    let output = Command::new(cli_bin())
        .args(["marketplace", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let id = stdout
        .lines()
        .find(|l| l.contains("redis"))
        .and_then(|l| l.split_whitespace().next())
        .expect("redis row should exist");

    let output = Command::new(cli_bin())
        .args(["marketplace", "uninstall", id])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(cli_bin())
        .args(["marketplace", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("redis"));
}
