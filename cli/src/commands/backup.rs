use clap::Subcommand;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/api/v1".to_string())
}

#[derive(Subcommand)]
pub enum BackupCommands {
    /// Create a snapshot of the cluster state
    Create,
    /// List available snapshots
    List,
    /// Restore a snapshot (wipes current state first)
    Restore {
        /// Snapshot filename (see `thiscloud backup list`)
        name: String,
    },
}

pub async fn run_backup_command(command: BackupCommands) -> anyhow::Result<()> {
    let client = api_client();
    let base = api_url();

    match command {
        BackupCommands::Create => {
            match client.post(format!("{}/backup", base)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let info: serde_json::Value = resp.json().await?;
                    println!("Backup created: {}", info["name"].as_str().unwrap_or("?"));
                    println!(
                        "  entries: {}   size: {} bytes",
                        info["entries"].as_u64().unwrap_or(0),
                        info["size_bytes"].as_u64().unwrap_or(0)
                    );
                }
                Ok(resp) => {
                    anyhow::bail!("failed to create backup: {}", api_error_message(resp).await)
                }
                Err(e) => anyhow::bail!("backup create failed: {}", e),
            }
        }
        BackupCommands::List => {
            match client.get(format!("{}/backup", base)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let snapshots: Vec<serde_json::Value> = resp.json().await?;
                    if snapshots.is_empty() {
                        println!("No backups found");
                    }
                    for s in snapshots {
                        println!(
                            "{:<36} {} bytes",
                            s["name"].as_str().unwrap_or("?"),
                            s["size_bytes"].as_u64().unwrap_or(0)
                        );
                    }
                }
                Ok(resp) => anyhow::bail!("failed to list backups: {}", api_error_message(resp).await),
                Err(e) => anyhow::bail!("backup list failed: {}", e),
            }
        }
        BackupCommands::Restore { name } => {
            match client.post(format!("{}/backup/{}/restore", base, name)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let info: serde_json::Value = resp.json().await?;
                    println!("Restored {} ({} entries)", name, info["entries"].as_u64().unwrap_or(0));
                }
                Ok(resp) => {
                    anyhow::bail!("failed to restore {}: {}", name, api_error_message(resp).await)
                }
                Err(e) => anyhow::bail!("backup restore failed: {}", e),
            }
        }
    }

    Ok(())
}