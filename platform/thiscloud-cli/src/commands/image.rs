use clap::Subcommand;
use serde_json::json;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/api/v1".to_string())
}

#[derive(Subcommand)]
pub enum ImageCommands {
    /// List all registered images
    List,
    /// Show details of an image
    Show {
        /// Image id or name
        image: String,
    },
    /// Register a new image
    Register {
        /// Image name
        #[arg(long)]
        name: String,
        /// Source reference: HTTP(S) URL or local pool path
        #[arg(long)]
        source: String,
        /// Disk format: qcow2 | iso | raw | cloud-init
        #[arg(long, default_value = "qcow2")]
        format: String,
        /// OS family: generic | ubuntu | debian | fedora | alma | rocky
        #[arg(long, default_value = "generic")]
        os_family: String,
        /// Version tag
        #[arg(long, default_value = "latest")]
        version: String,
        /// Expected SHA-256 checksum
        #[arg(long)]
        sha256: Option<String>,
        /// Register as a reusable template
        #[arg(long)]
        template: bool,
    },
    /// Mark an image as a reusable template (true/false)
    Template {
        /// Image id or name
        image: String,
        /// Template flag
        #[arg(long, default_value = "true")]
        template: bool,
    },
    /// Delete an image
    Delete {
        /// Image id or name
        image: String,
    },
}

pub async fn run_image_command(command: ImageCommands) -> anyhow::Result<()> {
    let client = api_client();
    let base = api_url();

    match command {
        ImageCommands::List => {
            let resp = client.get(format!("{}/images", base)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let images: Vec<serde_json::Value> = resp.json().await?;
            if images.is_empty() {
                println!("No images found");
                return Ok(());
            }
            println!(
                "{:<40} {:<16} {:<10} {:<10} {:<12} {:<6}",
                "ID", "NAME", "FORMAT", "OS", "VERSION", "TMPL"
            );
            for img in images {
                println!(
                    "{:<40} {:<16} {:<10} {:<10} {:<12} {:<6}",
                    img["id"].as_str().unwrap_or(""),
                    img["name"].as_str().unwrap_or(""),
                    img["format"].as_str().unwrap_or(""),
                    img["os_family"].as_str().unwrap_or(""),
                    img["version"].as_str().unwrap_or(""),
                    img["template"].as_bool().unwrap_or(false),
                );
            }
        }
        ImageCommands::Show { image } => {
            let resp = client
                .get(format!("{}/images/{}", base, image))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let img: serde_json::Value = resp.json().await?;
            println!("ID:       {}", img["id"].as_str().unwrap_or(""));
            println!("Name:     {}", img["name"].as_str().unwrap_or(""));
            println!("Source:   {}", img["source"].as_str().unwrap_or(""));
            println!("SHA-256:  {}", img["sha256"].as_str().unwrap_or(""));
            println!("Format:   {}", img["format"].as_str().unwrap_or(""));
            println!("OS:       {}", img["os_family"].as_str().unwrap_or(""));
            println!("Version:  {}", img["version"].as_str().unwrap_or(""));
            println!("Template: {}", img["template"].as_bool().unwrap_or(false));
            println!("Status:   {}", img["status"].as_str().unwrap_or(""));
        }
        ImageCommands::Register {
            name,
            source,
            format,
            os_family,
            version,
            sha256,
            template,
        } => {
            let body = json!({
                "name": name,
                "source": source,
                "format": format,
                "os_family": os_family,
                "version": version,
                "sha256": sha256.unwrap_or_default(),
                "template": template,
            });
            let resp = client
                .post(format!("{}/images", base))
                .json(&body)
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Registered image: {}", name);
        }
        ImageCommands::Template { image, template } => {
            let resp = client
                .put(format!("{}/images/{}/template", base, image))
                .json(&json!({ "template": template }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Image {} template flag set to {}", image, template);
        }
        ImageCommands::Delete { image } => {
            let resp = client
                .delete(format!("{}/images/{}", base, image))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Deleted image: {}", image);
        }
    }

    Ok(())
}