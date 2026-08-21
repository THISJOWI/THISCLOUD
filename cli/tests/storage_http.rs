use std::process::Command;
use std::sync::Arc;

use thiscloudd::storage::http::{app, StorageApiState};
use thiscloudd::storage::{MemoryStorageStore, MockStorageBackend, StorageModule};

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_thiscloud")
}

struct ApiServer {
    base_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl ApiServer {
    async fn start() -> Self {
        let module = StorageModule::new(
            Box::new(MockStorageBackend::new()),
            Box::new(MemoryStorageStore::default()),
        );
        let state = StorageApiState::new(Arc::new(tokio::sync::Mutex::new(module)));
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
async fn test_cli_storage_create_and_list() {
    let server = ApiServer::start().await;

    let output = Command::new(cli_bin())
        .args([
            "storage",
            "create",
            "--name",
            "data",
            "--pool-type",
            "linstor",
            "--replication",
            "2",
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
    assert!(stdout.contains("data"));

    let output = Command::new(cli_bin())
        .args(["storage", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("data"));
    assert!(stdout.contains("linstor"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cli_storage_delete() {
    let server = ApiServer::start().await;

    Command::new(cli_bin())
        .args([
            "storage",
            "create",
            "--name",
            "deldpool",
            "--pool-type",
            "local",
        ])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();

    let output = Command::new(cli_bin())
        .args(["storage", "delete", "deldpool"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(cli_bin())
        .args(["storage", "list"])
        .env("THISCLOUD_API_URL", &server.base_url)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("deldpool"));
}
