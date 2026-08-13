use clap::Subcommand;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/api/v1".to_string())
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
            println!("{:<20} {:<12} {:<10} {:<10} {:<8} {:<10}", "ID", "NAME", "ROLE", "STATE", "CPUS", "MEMORY_MB");
            for n in nodes {
                println!(
                    "{:<20} {:<12} {:<10} {:<10} {:<8} {:<10}",
                    n["id"].as_str().unwrap_or(""),
                    n["name"].as_str().unwrap_or(""),
                    n["role"].as_str().unwrap_or(""),
                    n["state"].as_str().unwrap_or(""),
                    n["cpus_used"].as_u64().unwrap_or(0),
                    n["memory_used_mb"].as_u64().unwrap_or(0),
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
            println!("ID:          {}", n["id"].as_str().unwrap_or(""));
            println!("Name:        {}", n["name"].as_str().unwrap_or(""));
            println!("Role:        {}", n["role"].as_str().unwrap_or(""));
            println!("Address:     {}", n["address"].as_str().unwrap_or(""));
            println!("Hostname:    {}", n["hostname"].as_str().unwrap_or(""));
            println!("State:       {}", n["state"].as_str().unwrap_or(""));
            println!("CPUs:        {}/{}", n["cpus_used"].as_u64().unwrap_or(0), n["cpus_total"].as_u64().unwrap_or(0));
            println!("Memory:      {}/{} MB", n["memory_used_mb"].as_u64().unwrap_or(0), n["memory_total_mb"].as_u64().unwrap_or(0));
            println!("VMs:         {}", n["vms"].as_u64().unwrap_or(0));
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