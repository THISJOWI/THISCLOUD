use clap::Subcommand;
use serde_json::json;

use super::{api_client, api_error_message};

fn api_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/api/v1".to_string())
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
        /// Cloud-init user-data (cloud-config), applied at first boot
        #[arg(long)]
        cloud_init: Option<String>,
        /// Boot with UEFI firmware (OVMF)
        #[arg(long)]
        uefi: bool,
        /// Attach a vTPM device (requires UEFI)
        #[arg(long)]
        tpm: bool,
        /// Mark VM as a reusable template
        #[arg(long)]
        template: bool,
        /// Place the VM on a specific node (empty = best-fit scheduler)
        #[arg(long)]
        node: Option<String>,
        /// Scheduler affinity label (can be repeated)
        #[arg(long = "affinity")]
        affinity: Vec<String>,
        /// Scheduler anti-affinity label (can be repeated)
        #[arg(long = "anti-affinity")]
        anti_affinity: Vec<String>,
        /// Boot from a registered image (name or id). Derives disk_path when omitted
        #[arg(long)]
        image: Option<String>,
    },
    /// Take a snapshot of a VM
    Snapshot {
        /// VM ID or name
        vm: String,
        /// Snapshot name
        #[arg(long)]
        name: String,
    },
    /// Restore a VM from a snapshot
    Restore {
        /// VM ID or name
        vm: String,
        /// Snapshot ID to restore
        #[arg(long)]
        snapshot_id: String,
    },
    /// Clone a VM or template into a new VM
    Clone {
        /// Source VM ID or name
        vm: String,
        /// New VM name
        #[arg(long)]
        name: String,
    },
    /// Resize a VM (cpus / memory)
    Resize {
        /// VM ID or name
        vm: String,
        /// New number of vCPUs (0 = unchanged)
        #[arg(long, default_value = "0")]
        cpus: u32,
        /// New memory in MB (0 = unchanged)
        #[arg(long, default_value = "0")]
        memory: u32,
    },
    /// Attach a data disk to a VM
    AttachDisk {
        /// VM ID or name
        vm: String,
        /// Disk path (qcow2)
        #[arg(long)]
        path: String,
        /// Disk size in GB
        #[arg(long, default_value = "0")]
        size_gb: u32,
    },
    /// Detach a data disk from a VM
    DetachDisk {
        /// VM ID or name
        vm: String,
        /// Disk ID to detach
        #[arg(long)]
        disk_id: String,
    },
    /// Attach a NIC to a VM
    AttachNic {
        /// VM ID or name
        vm: String,
        /// Tap/network name
        #[arg(long)]
        tap: String,
    },
    /// Detach a NIC from a VM
    DetachNic {
        /// VM ID or name
        vm: String,
        /// Tap/network name
        #[arg(long)]
        tap: String,
    },
    /// Show console (VNC/vsock) access URL for a VM
    Console {
        /// VM ID or name
        vm: String,
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
            println!("UEFI:        {}", v["uefi"].as_bool().unwrap_or(false));
            println!("TPM:         {}", v["tpm"].as_bool().unwrap_or(false));
            println!("Template:    {}", v["template"].as_bool().unwrap_or(false));
            if let Some(ci) = v["cloud_init"].as_str() {
                println!("Cloud-init:  {}", ci);
            }
            if let Some(nets) = v["networks"].as_array() {
                let names: Vec<&str> = nets.iter().filter_map(|n| n.as_str()).collect();
                println!("Networks:    {}", names.join(", "));
            }
            if let Some(disks) = v["disks"].as_array() {
                for d in disks {
                    println!(
                        "Disk:        {} ({} GB) path={}",
                        d["id"].as_str().unwrap_or(""),
                        d["size_gb"].as_u64().unwrap_or(0),
                        d["path"].as_str().unwrap_or("")
                    );
                }
            }
            if let Some(snaps) = v["snapshots"].as_array() {
                for s in snaps {
                    println!(
                        "Snapshot:    {} name={} at={}",
                        s["id"].as_str().unwrap_or(""),
                        s["name"].as_str().unwrap_or(""),
                        s["created_at"].as_str().unwrap_or("")
                    );
                }
            }
        }
        VmCommands::Create {
            name,
            cpus,
            memory,
            disk,
            kernel,
            kernel_args,
            networks,
            cloud_init,
            uefi,
            tpm,
            template,
            node,
            affinity,
            anti_affinity,
            image,
        } => {
            let disk_path = disk.unwrap_or_else(|| format!("/var/lib/thiscloud/vms/{}.qcow2", name));
            let mut body = json!({
                "name": name,
                "cpus": cpus,
                "memory_mb": memory,
                "disk_path": disk_path,
                "networks": networks,
                "uefi": uefi,
                "tpm": tpm,
                "template": template,
                "affinity": affinity,
                "anti_affinity": anti_affinity,
            });
            if let Some(img) = image {
                body["image"] = json!(img);
            }
            if let Some(n) = node {
                body["node"] = json!(n);
            }
            if let Some(k) = kernel {
                body["kernel"] = json!(k);
            }
            if let Some(ka) = kernel_args {
                body["kernel_args"] = json!(ka);
            }
            if let Some(ci) = cloud_init {
                body["cloud_init"] = json!(ci);
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
        VmCommands::Snapshot { vm, name } => {
            let resp = client
                .post(format!("{}/vms/{}/snapshot", base, vm))
                .json(&json!({ "name": name }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Snapshot taken: {} ({}).", name, vm);
        }
        VmCommands::Restore { vm, snapshot_id } => {
            let resp = client
                .post(format!("{}/vms/{}/restore", base, vm))
                .json(&json!({ "snapshot_id": snapshot_id }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("VM restored from snapshot {}: {}", snapshot_id, vm);
        }
        VmCommands::Clone { vm, name } => {
            let resp = client
                .post(format!("{}/vms/{}/clone", base, vm))
                .json(&json!({ "name": name }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Cloned VM: {} -> {}", vm, name);
        }
        VmCommands::Resize { vm, cpus, memory } => {
            let resp = client
                .post(format!("{}/vms/{}/resize", base, vm))
                .json(&json!({ "cpus": cpus, "memory_mb": memory }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("VM resized: {}", vm);
        }
        VmCommands::AttachDisk {
            vm,
            path,
            size_gb,
        } => {
            let resp = client
                .put(format!("{}/vms/{}/disks", base, vm))
                .json(&json!({ "path": path, "size_gb": size_gb }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Disk attached to {}: {}", vm, path);
        }
        VmCommands::DetachDisk { vm, disk_id } => {
            let resp = client
                .delete(format!("{}/vms/{}/disks/{}", base, vm, disk_id))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("Disk detached from {}: {}", vm, disk_id);
        }
        VmCommands::AttachNic { vm, tap } => {
            let resp = client
                .put(format!("{}/vms/{}/nics", base, vm))
                .json(&json!({ "tap": tap }))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("NIC attached to {}: {}", vm, tap);
        }
        VmCommands::DetachNic { vm, tap } => {
            let resp = client
                .delete(format!("{}/vms/{}/nics/{}", base, vm, tap))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            println!("NIC detached from {}: {}", vm, tap);
        }
        VmCommands::Console { vm } => {
            let resp = client
                .get(format!("{}/vms/{}/console", base, vm))
                .send()
                .await?;
            if !resp.status().is_success() {
                let msg = api_error_message(resp).await;
                anyhow::bail!("API error: {}", msg);
            }
            let v: serde_json::Value = resp.json().await?;
            println!("Console URL: {}", v["url"].as_str().unwrap_or(""));
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
