use super::api_client;

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
    Ok(())
}