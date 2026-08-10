use clap::Subcommand;
use serde_json::json;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

#[derive(Subcommand)]
pub enum MarketplaceCommands {
    /// List all marketplace apps
    List,
    /// Install a marketplace app
    Install {
        /// App name
        #[arg(long)]
        name: String,
        /// App type: iso | docker | cloud-init | turbokit
        #[arg(long, default_value = "docker")]
        app_type: String,
        /// Source reference (image name or URL)
        #[arg(long)]
        source: String,
        /// Version
        #[arg(long, default_value = "latest")]
        version: String,
        /// Description
        #[arg(long)]
        description: Option<String>,
    },
    /// Uninstall a marketplace app by id
    Uninstall {
        /// App id
        id: String,
    },
}

pub async fn run_marketplace_command(command: MarketplaceCommands) -> anyhow::Result<()> {
    let client = api_client();
    let base = api_url();

    match command {
        MarketplaceCommands::List => {
            let resp = client
                .get(format!("{}/marketplace/apps", base))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let apps: Vec<serde_json::Value> = resp.json().await?;
            if apps.is_empty() {
                println!("No marketplace apps found");
                return Ok(());
            }
            println!(
                "{:<40} {:<16} {:<16} {:<12} {:<6}",
                "ID", "NAME", "TYPE", "VERSION", "STATUS"
            );
            for app in apps {
                println!(
                    "{:<40} {:<16} {:<16} {:<12} {:<6}",
                    app["id"].as_str().unwrap_or(""),
                    app["name"].as_str().unwrap_or(""),
                    app["app_type"].as_str().unwrap_or(""),
                    app["version"].as_str().unwrap_or(""),
                    app["status"].as_str().unwrap_or(""),
                );
            }
        }
        MarketplaceCommands::Install {
            name,
            app_type,
            source,
            version,
            description,
        } => {
            let body = json!({
                "name": name,
                "app_type": app_type,
                "source": source,
                "version": version,
                "description": description.unwrap_or_default(),
            });
            let resp = client
                .post(format!("{}/marketplace/apps", base))
                .json(&body)
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Installed: {}", name);
        }
        MarketplaceCommands::Uninstall { id } => {
            let resp = client
                .delete(format!("{}/marketplace/apps/{}", base, id))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Uninstalled: {}", id);
        }
    }

    Ok(())
}
