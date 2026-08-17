use super::api_client;
use serde_json::Value;

fn api_base() -> String {
    let raw = std::env::var("THISCLOUD_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080/api/v1".to_string());
    let trimmed = raw.trim_end_matches('/').to_string();
    // Accept both a raw base (http://host:port) and the versioned root
    // (http://host:port/api/v1). Other commands expect the versioned root.
    if trimmed.ends_with("/api/v1") {
        trimmed
    } else {
        format!("{}/api/v1", trimmed)
    }
}

fn health_url() -> String {
    format!("{}/healthz", api_base())
}

pub async fn run_status() -> anyhow::Result<()> {
    println!("THISCLOUD Cluster Status");
    println!("========================");

    let client = api_client();

    // 1. Daemon liveness via the health endpoint.
    let health = health_url();
    let daemon_up = match client.get(&health).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };
    if daemon_up {
        println!("Daemon:   Running ({})", health);
    } else {
        println!("Daemon:   Not running ({})", health);
        println!("\nRun: thiscloud init, or check thiscloudd.service");
        return Ok(());
    }

    let base = api_base();

    // 1b. Readiness: dependencies the daemon actually uses (etcd etc).
    let ready = format!("{}/ready", base);
    match client.get(&ready).send().await {
        Ok(resp) => {
            let status = resp.status();
            let mut checks = String::new();
            if let Ok(body) = resp.json::<Value>().await {
                if let Some(checks_obj) = body["checks"].as_object() {
                    let parts: Vec<String> = checks_obj
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v.as_bool().unwrap_or(false)))
                        .collect();
                    checks = format!(" ({})", parts.join(", "));
                }
            }
            let label = if status.is_success() { "Ready" } else { "Degraded" };
            println!("Readiness: {}{}", label, checks);
        }
        Err(_) => println!("Readiness: unknown"),
    }

    // 2. Cluster nodes with live state, when the daemon exposes them.
    match client.get(format!("{}/nodes", base)).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(nodes) = resp.json::<Vec<Value>>().await {
                let online = nodes
                    .iter()
                    .filter(|n| n["state"].as_str().unwrap_or("") == "online")
                    .count();
                println!(
                    "Nodes:    {} ({} online, {} offline)",
                    nodes.len(),
                    online,
                    nodes.len() - online
                );
                for n in nodes {
                    let name = n["name"].as_str().unwrap_or("unknown");
                    let state = n["state"].as_str().unwrap_or("unknown");
                    let role = n["role"].as_str().unwrap_or("unknown");
                    let vms = n["vms"].as_u64().unwrap_or(0);
                    let ip = n["address"].as_str().unwrap_or("?");
                    println!(
                        "  {:12} {:<8} {:<8} vms={:<3} {}",
                        name, state, role, vms, ip
                    );
                }
            }
        }
        Ok(_) => println!("Nodes:    unavailable"),
        Err(_) => println!("Nodes:    unavailable (daemon may be degraded)"),
    }

    // 3. VM inventory with per-VM state.
    match client.get(format!("{}/vms", base)).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(vms) = resp.json::<Vec<Value>>().await {
                let running = vms
                    .iter()
                    .filter(|v| v["status"].as_str().unwrap_or("") == "running")
                    .count();
                println!(
                    "VMs:      {} ({} running, {} stopped)",
                    vms.len(),
                    running,
                    vms.len() - running
                );
                for v in vms {
                    let name = v["name"].as_str().unwrap_or("?");
                    let status = v["status"].as_str().unwrap_or("unknown");
                    let node = v["node"].as_str().unwrap_or("?");
                    let cpus = v["cpus"].as_u64().unwrap_or(0);
                    let mem = v["memory_mb"].as_u64().unwrap_or(0);
                    println!(
                        "  {:20} {:<8} node={:<10} cpus={} mem={}MB",
                        name, status, node, cpus, mem
                    );
                }
            }
        }
        Ok(_) => println!("VMs:      unavailable"),
        Err(_) => println!("VMs:      unavailable (daemon may be degraded)"),
    }

    Ok(())
}
