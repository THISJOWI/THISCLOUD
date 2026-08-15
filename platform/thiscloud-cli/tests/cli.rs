use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;
use tempfile::TempDir;

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_thiscloud")
}

#[test]
fn test_cli_init_creates_config() {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("config");
    let data_dir = tmp.path().join("data");

    let output = Command::new(cli_bin())
        .args(["init", "--ip", "10.0.0.10", "--role", "master"])
        .env("THISCLOUD_CONFIG_DIR", &config_dir)
        .env("THISCLOUD_DATA_DIR", &data_dir)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = config_dir.join("config.toml");
    assert!(config_path.exists(), "config.toml was not created");

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("10.0.0.10"));
    assert!(content.contains("master"));

    assert!(data_dir.join("vms").exists());
    assert!(data_dir.join("storage").exists());
}

#[test]
fn test_cli_status_after_init() {
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("config");

    let init = Command::new(cli_bin())
        .args(["init"])
        .env("THISCLOUD_CONFIG_DIR", &config_dir)
        .env("THISCLOUD_DATA_DIR", tmp.path().join("data"))
        .output()
        .unwrap();
    assert!(init.status.success());

    let output = Command::new(cli_bin())
        .arg("status")
        .env("THISCLOUD_CONFIG_DIR", &config_dir)
        .env("THISCLOUD_DATA_DIR", tmp.path().join("data"))
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("THISCLOUD Cluster Status"));
    assert!(stdout.contains("thiscloud-cluster"));
}

#[test]
fn test_cli_status_without_config() {
    let tmp = TempDir::new().unwrap();

    let output = Command::new(cli_bin())
        .arg("status")
        .env("THISCLOUD_CONFIG_DIR", tmp.path().join("nonexistent"))
        .env("THISCLOUD_DATA_DIR", tmp.path().join("data"))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No configuration found"));
}

/// Minimal HTTP responder that answers a single POST /api/v1/nodes with a 201
/// registration response, so the CLI can be exercised end-to-end.
fn mock_master_server() -> String {
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        tx.send(addr.to_string()).unwrap();

        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 2048];
        let _ = stream.read(&mut buf);
        let body = r#"{"id":"node-join-test","name":"mock-host","role":"worker"}"#;
        let response = format!(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    rx.recv().unwrap()
}

#[test]
fn test_cli_join_writes_node_identity() {
    let master = mock_master_server();
    let tmp = TempDir::new().unwrap();
    let config_dir = tmp.path().join("config");

    // `init` creates the config file join will append to.
    let init = Command::new(cli_bin())
        .args(["init", "--ip", "192.168.1.18", "--role", "worker"])
        .env("THISCLOUD_CONFIG_DIR", &config_dir)
        .env("THISCLOUD_DATA_DIR", tmp.path().join("data"))
        .output()
        .unwrap();
    assert!(init.status.success());

    let output = Command::new(cli_bin())
        .args(["join", "--master", &format!("http://{master}"), "--ip", "192.168.1.18"])
        .env("THISCLOUD_CONFIG_DIR", &config_dir)
        .env("THISCLOUD_DATA_DIR", tmp.path().join("data"))
        .env("HOSTNAME", "mock-host")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "join failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = config_dir.join("config.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[node]"), "config.toml:\n{}", content);
    assert!(content.contains("node-join-test"), "config.toml:\n{}", content);
    assert!(
        content.contains(&format!("http://{master}")),
        "config.toml:\n{}",
        content
    );
}
