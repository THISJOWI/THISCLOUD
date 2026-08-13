use clap::Subcommand;
use serde_json::json;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/api/v1".to_string())
}

#[derive(Subcommand)]
pub enum NetworkCommands {
    /// List all networks
    List,
    /// Create a new network
    Create {
        /// Network name
        #[arg(long)]
        name: String,
        /// CIDR (e.g. 10.0.0.0/24)
        #[arg(long)]
        cidr: String,
        /// Gateway IP
        #[arg(long)]
        gateway: Option<String>,
        /// VLAN id
        #[arg(long)]
        vlan: Option<u16>,
    },
    /// Delete a network by id
    Delete {
        /// Network id
        id: String,
    },
}

pub async fn run_network_command(command: NetworkCommands) -> anyhow::Result<()> {
    let client = api_client();
    let base = api_url();

    match command {
        NetworkCommands::List => {
            let resp = client.get(format!("{}/networks", base)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let networks: Vec<serde_json::Value> = resp.json().await?;
            if networks.is_empty() {
                println!("No networks found");
                return Ok(());
            }
            println!(
                "{:<16} {:<12} {:<18} {:<12}",
                "ID", "NAME", "CIDR", "GATEWAY"
            );
            for net in networks {
                println!(
                    "{:<16} {:<12} {:<18} {:<12}",
                    net["id"].as_str().unwrap_or(""),
                    net["name"].as_str().unwrap_or(""),
                    net["cidr"].as_str().unwrap_or(""),
                    net["gateway"].as_str().unwrap_or(""),
                );
            }
        }
        NetworkCommands::Create {
            name,
            cidr,
            gateway,
            vlan,
        } => {
            let mut body = json!({
                "name": name,
                "cidr": cidr,
                "gateway": gateway.unwrap_or_else(|| "10.0.0.1".to_string()),
            });
            if let Some(v) = vlan {
                body["vlan"] = serde_json::Value::Number(v.into());
            }
            let resp = client
                .post(format!("{}/networks", base))
                .json(&body)
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Created network: {}", name);
        }
        NetworkCommands::Delete { id } => {
            let resp = client
                .delete(format!("{}/networks/{}", base, id))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Deleted network: {}", id);
        }
    }

    Ok(())
}
