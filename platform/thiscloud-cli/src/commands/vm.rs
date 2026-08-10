use clap::Subcommand;
use serde_json::json;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

#[derive(Subcommand)]
pub enum VmCommands {
    /// List all VMs
    List,
    /// Show details of a VM
    Show {
        /// VM ID or name
        vm: String,
    },
    /// Create a new VM
    Create {
        /// VM name
        #[arg(long)]
        name: String,
        /// Number of vCPUs
        #[arg(long, default_value = "1")]
        cpus: u32,
        /// Memory in MB
        #[arg(long, default_value = "1024")]
        memory: u32,
        /// Disk path (qcow2). Auto-generated at /var/lib/thiscloud/vms/<name>.qcow2 if omitted
        #[arg(long)]
        disk: Option<String>,
        /// Kernel binary path for cloud-hypervisor
        #[arg(long)]
        kernel: Option<String>,
        /// Kernel boot arguments
        #[arg(long)]
        kernel_args: Option<String>,
        /// Network name or ID (can be repeated)
        #[arg(long = "network")]
        networks: Vec<String>,
    },
    /// Start a VM
    Start {
        /// VM ID or name
        vm: String,
    },
    /// Stop a VM
    Stop {
        /// VM ID or name
        vm: String,
    },
    /// Delete a VM
    Delete {
        /// VM ID or name
        vm: String,
    },
}

pub async fn run_vm_command(command: VmCommands) -> anyhow::Result<()> {
    let client = api_client();
    let base = api_url();

    match command {
        VmCommands::List => {
            let resp = client.get(format!("{}/vms", base)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let vms: Vec<serde_json::Value> = resp.json().await?;
            if vms.is_empty() {
                println!("No VMs found");
                return Ok(());
            }
            println!("{:<20} {:<12} {:<6} {:<10}", "ID", "NAME", "CPUS", "STATUS");
            for vm in vms {
                println!(
                    "{:<20} {:<12} {:<6} {:<10}",
                    vm["id"].as_str().unwrap_or(""),
                    vm["name"].as_str().unwrap_or(""),
                    vm["cpus"].as_u64().unwrap_or(0),
                    vm["status"].as_str().unwrap_or(""),
                );
            }
        }
        VmCommands::Show { vm } => {
            let resp = client.get(format!("{}/vms/{}", base, vm)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let v: serde_json::Value = resp.json().await?;
            println!("ID:          {}", v["id"].as_str().unwrap_or(""));
            println!("Name:        {}", v["name"].as_str().unwrap_or(""));
            println!("CPUs:        {}", v["cpus"].as_u64().unwrap_or(0));
            println!("Memory:      {} MB", v["memory_mb"].as_u64().unwrap_or(0));
            println!("Disk:        {}", v["disk_path"].as_str().unwrap_or(""));
            println!("Kernel:      {}", v["kernel"].as_str().unwrap_or(""));
            println!("Kernel args: {}", v["kernel_args"].as_str().unwrap_or(""));
            println!("Status:      {}", v["status"].as_str().unwrap_or(""));
            if let Some(nets) = v["networks"].as_array() {
                let names: Vec<&str> = nets.iter().filter_map(|n| n.as_str()).collect();
                println!("Networks:    {}", names.join(", "));
            }
        }
        VmCommands::Create { name, cpus, memory, disk, kernel, kernel_args, networks } => {
            let disk_path = disk.unwrap_or_else(|| format!("/var/lib/thiscloud/vms/{}.qcow2", name));
            let mut body = json!({
                "name": name,
                "cpus": cpus,
                "memory_mb": memory,
                "disk_path": disk_path,
                "networks": networks,
            });
            if let Some(k) = kernel {
                body["kernel"] = json!(k);
            }
            if let Some(ka) = kernel_args {
                body["kernel_args"] = json!(ka);
            }
            let resp = client
                .post(format!("{}/vms", base))
                .json(&body)
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Created VM: {}", name);
        }
        VmCommands::Start { vm } => {
            let resp = client
                .post(format!("{}/vms/{}/start", base, vm))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Started VM: {}", vm);
        }
        VmCommands::Stop { vm } => {
            let resp = client
                .post(format!("{}/vms/{}/stop", base, vm))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Stopped VM: {}", vm);
        }
        VmCommands::Delete { vm } => {
            let resp = client.delete(format!("{}/vms/{}", base, vm)).send().await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Deleted VM: {}", vm);
        }
    }

    Ok(())
}
