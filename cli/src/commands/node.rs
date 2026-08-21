use clap::Subcommand;
use std::io::IsTerminal;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/api/v1".to_string())
}

fn color_enabled() -> bool {
    if std::env::var("NO_COLOR").map(|v| !v.is_empty()).unwrap_or(false) {
        return false;
    }
    std::io::stdout().is_terminal()
}

fn paint(code: &str, s: &str) -> String {
    if color_enabled() {
        format!("\x1b[{}m{}\x1b[0m", code, s)
    } else {
        s.to_string()
    }
}

fn state_color(state: &str) -> &'static str {
    match state {
        "online" => "32",
        "offline" => "31",
        "draining" => "33",
        _ => "0",
    }
}

/// Humanize memory (MB) as a compact string.
fn humanize_memory(mb: u64) -> String {
    if mb == 0 {
        return "0".to_string();
    }
    if mb >= 1024 {
        format!("{:.1}G", mb as f64 / 1024.0)
    } else {
        format!("{}M", mb)
    }
}

/// Humanize a UNIX timestamp as seconds/minutes/hours/days ago.
fn humanize_ago(last_seen_secs: u64) -> String {
    if last_seen_secs == 0 {
        return "-".to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let elapsed = now.saturating_sub(last_seen_secs);
    if elapsed < 2 {
        "now".to_string()
    } else if elapsed < 60 {
        format!("{}s", elapsed)
    } else if elapsed < 3600 {
        format!("{}m", elapsed / 60)
    } else if elapsed < 86400 {
        format!("{}h", elapsed / 3600)
    } else {
        format!("{}d", elapsed / 86400)
    }
}

#[derive(Subcommand)]
pub enum NodeCommands {
    /// List cluster nodes
    List,
    /// Show a node's details
    Show {
        /// Node ID
        node: String,
    },
    /// Register a node with the master
    Register {
        /// Node name (hostname)
        #[arg(long)]
        name: String,
        /// Node agent address (ip:port)
        #[arg(long)]
        address: String,
        /// Role: master or worker
        #[arg(long, default_value = "worker")]
        role: String,
        /// Total CPUs (0 = unknown)
        #[arg(long, default_value = "0")]
        cpus: u32,
        /// Total memory in MB (0 = unknown)
        #[arg(long, default_value = "0")]
        memory_mb: u32,
        /// Affinity/scheduling labels (can be repeated)
        #[arg(long = "label")]
        labels: Vec<String>,
    },
    /// Remove a node from the cluster
    Remove {
        /// Node ID
        node: String,
    },
    /// Start draining a node (excluded from scheduling)
    Drain {
        /// Node ID
        node: String,
    },
    /// Stop draining a node
    Undrain {
        /// Node ID
        node: String,
    },
}

pub async fn run_node_command(command: NodeCommands) -> anyhow::Result<()> {
    let client = api_client();
    let base = api_url();

    match command {
        NodeCommands::List => {
            let resp = client.get(format!("{}/nodes", base)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let nodes: Vec<serde_json::Value> = resp.json().await?;
            if nodes.is_empty() {
                println!("No nodes found");
                return Ok(());
            }
            println!(
                "{:<20} {:<14} {:<8} {:<10} {:<12} {:<13} {:<7} {:<10}",
                "ID", "NAME", "ROLE", "STATE", "CPUS", "MEMORY", "DRAIN", "LAST SEEN"
            );
            for n in nodes {
                let id = n["id"].as_str().unwrap_or("");
                let name = n["name"].as_str().unwrap_or("");
                let role = n["role"].as_str().unwrap_or("");
                let state = n["state"].as_str().unwrap_or("");
                let cpus_used = n["cpus_used"].as_u64().unwrap_or(0);
                let cpus_total = n["cpus_total"].as_u64().unwrap_or(0);
                let mem_used = n["memory_used_mb"].as_u64().unwrap_or(0);
                let mem_total = n["memory_total_mb"].as_u64().unwrap_or(0);
                let drain = if state == "draining" { "yes" } else { "-" };
                let last_seen = humanize_ago(n["last_seen_secs"].as_u64().unwrap_or(0));
                println!(
                    "{:<20} {:<14} {:<8} {} {:<12} {:<13} {:<7} {:<10}",
                    id,
                    name,
                    role,
                    paint(state_color(state), &format!("{:<10}", state)),
                    if cpus_total > 0 {
                        format!("{}/{}", cpus_used, cpus_total)
                    } else {
                        format!("{}", cpus_used)
                    },
                    format!(
                        "{}/{}",
                        humanize_memory(mem_used),
                        humanize_memory(mem_total)
                    ),
                    drain,
                    last_seen,
                );
            }
        }
        NodeCommands::Show { node } => {
            let resp = client.get(format!("{}/nodes/{}", base, node)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let n: serde_json::Value = resp.json().await?;
            let state = n["state"].as_str().unwrap_or("");
            println!("ID:          {}", n["id"].as_str().unwrap_or(""));
            println!("Name:        {}", n["name"].as_str().unwrap_or(""));
            println!("Role:        {}", n["role"].as_str().unwrap_or(""));
            println!("Address:     {}", n["address"].as_str().unwrap_or(""));
            println!("Hostname:    {}", n["hostname"].as_str().unwrap_or(""));
            println!("State:       {}", paint(state_color(state), state));
            println!(
                "CPUs:        {}/{}",
                n["cpus_used"].as_u64().unwrap_or(0),
                n["cpus_total"].as_u64().unwrap_or(0)
            );
            println!(
                "Memory:      {}/{}",
                humanize_memory(n["memory_used_mb"].as_u64().unwrap_or(0)),
                humanize_memory(n["memory_total_mb"].as_u64().unwrap_or(0))
            );
            println!("VMs:         {}", n["vms"].as_u64().unwrap_or(0));
            println!(
                "Draining:    {}",
                if state == "draining" { "yes" } else { "no" }
            );
            println!(
                "Last seen:   {}",
                humanize_ago(n["last_seen_secs"].as_u64().unwrap_or(0))
            );
            if let Some(labels) = n["labels"].as_array() {
                let names: Vec<&str> = labels.iter().filter_map(|l| l.as_str()).collect();
                println!("Labels:      {}", names.join(", "));
            }
        }
        NodeCommands::Register {
            name,
            address,
            role,
            cpus,
            memory_mb,
            labels,
        } => {
            let body = serde_json::json!({
                "name": name,
                "address": address,
                "role": role,
                "cpus_total": cpus,
                "memory_total_mb": memory_mb,
                "labels": labels,
            });
            let resp = client.post(format!("{}/nodes", base)).json(&body).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let n: serde_json::Value = resp.json().await?;
            println!(
                "Registered node: {} ({})",
                n["name"].as_str().unwrap_or(""),
                n["id"].as_str().unwrap_or("")
            );
        }
        NodeCommands::Remove { node } => {
            let resp = client.delete(format!("{}/nodes/{}", base, node)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Removed node: {}", node);
        }
        NodeCommands::Drain { node } => {
            let resp = client
                .put(format!("{}/nodes/{}/drain", base, node))
                .json(&serde_json::json!({ "drain": true }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Node draining: {}", node);
        }
        NodeCommands::Undrain { node } => {
            let resp = client
                .put(format!("{}/nodes/{}/drain", base, node))
                .json(&serde_json::json!({ "drain": false }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Node back online: {}", node);
        }
    }

    Ok(())
}