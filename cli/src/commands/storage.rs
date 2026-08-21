use clap::Subcommand;
use serde_json::json;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/api/v1".to_string())
}

#[derive(Subcommand)]
pub enum StorageCommands {
    /// List all storage pools
    List,
    /// Create a new storage pool
    Create {
        /// Pool name
        #[arg(long)]
        name: String,
        /// Pool type: linstor | drbd | local
        #[arg(long, default_value = "linstor")]
        pool_type: String,
        /// Replication factor
        #[arg(long, default_value = "2")]
        replication: u32,
        /// Block devices (comma-separated)
        #[arg(long)]
        devices: Option<String>,
    },
    /// Delete a storage pool by name
    Delete {
        /// Pool name
        name: String,
    },
}

pub async fn run_storage_command(command: StorageCommands) -> anyhow::Result<()> {
    let client = api_client();
    let base = api_url();

    match command {
        StorageCommands::List => {
            let resp = client.get(format!("{}/storage/pools", base)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let pools: Vec<serde_json::Value> = resp.json().await?;
            if pools.is_empty() {
                println!("No storage pools found");
                return Ok(());
            }
            println!(
                "{:<16} {:<12} {:<10} {:<6}",
                "NAME", "TYPE", "DEVICES", "REPL"
            );
            for pool in pools {
                let devices = pool["devices"]
                    .as_array()
                    .map(|d| {
                        d.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default();
                println!(
                    "{:<16} {:<12} {:<10} {:<6}",
                    pool["name"].as_str().unwrap_or(""),
                    pool["pool_type"].as_str().unwrap_or(""),
                    devices,
                    pool["replication"].as_u64().unwrap_or(0),
                );
            }
        }
        StorageCommands::Create {
            name,
            pool_type,
            replication,
            devices,
        } => {
            let devices: Vec<String> = devices
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let body = json!({
                "name": name,
                "pool_type": pool_type,
                "devices": devices,
                "replication": replication,
            });
            let resp = client
                .post(format!("{}/storage/pools", base))
                .json(&body)
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Created storage pool: {}", name);
        }
        StorageCommands::Delete { name } => {
            let resp = client
                .delete(format!("{}/storage/pools/{}", base, name))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Deleted storage pool: {}", name);
        }
    }

    Ok(())
}
