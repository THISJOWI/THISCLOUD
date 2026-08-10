use std::fs;
use std::process::Command;
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
