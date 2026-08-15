use super::api_client;
use std::fs;
use std::path::PathBuf;

fn config_path() -> PathBuf {
    std::env::var("THISCLOUD_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/etc/thiscloud"))
        .join("config.toml")
}

pub async fn run_join(master: &str, ip: Option<&str>) -> anyhow::Result<()> {
    let ip = ip.unwrap_or("127.0.0.1");

    println!("Joining THISCLOUD cluster at {}...", master);
    println!("  Local IP: {}", ip);

    // T1.3: register this worker node with the master over its API.
    let base = format!("{}/api/v1", master.trim_end_matches('/'));
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string());
    let cpus = std::thread::available_parallelism()
        .map(|p| p.get() as u32)
        .unwrap_or(1);

    let body = serde_json::json!({
        "name": hostname,
        "address": format!("{}:8080", ip),
        "role": "worker",
        "cpus_total": cpus,
        "memory_total_mb": 0,
        "labels": [],
    });

    let client = api_client();
    let resp = client
        .post(format!("{}/nodes", base))
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        anyhow::bail!("Master rejected node registration: {}", status);
    }
    let node: serde_json::Value = resp.json().await?;
    println!(
        "Joined cluster. Registered as worker: {} ({})",
        node["name"].as_str().unwrap_or(""),
        node["id"].as_str().unwrap_or("")
    );
    println!("  Role: worker | Agent address: {}:8080", ip);

    // Persist the assigned node id + master so the local daemon can run the
    // self-heartbeat loop against the master.
    let node_id = node["id"].as_str().unwrap_or("").to_string();
    if node_id.is_empty() {
        anyhow::bail!("Master response did not include a node id");
    }
    let config_path = config_path();
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let mut value: toml::Value = content.parse()?;
        let table = value
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("config.toml is not a table"))?;
        let node_cfg = table
            .entry("node")
            .or_insert_with(|| toml::Value::Table(Default::default()));
        if let toml::Value::Table(t) = node_cfg {
            t.insert("id".to_string(), toml::Value::String(node_id));
            t.insert(
                "master".to_string(),
                toml::Value::String(master.trim_end_matches('/').to_string()),
            );
        } else {
            anyhow::bail!("[node] section is not a table");
        }
        fs::write(&config_path, toml::to_string_pretty(&value)?)?;
        println!("  Node identity saved to {}", config_path.display());
    } else {
        println!("  (no local config.toml found; node identity not saved)");
    }

    Ok(())
}