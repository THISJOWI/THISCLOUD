use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

mod slot;
mod verify;
mod boot;

const SLOTS_DIR: &str = "/var/lib/thpkg/slots";
const ACTIVE_SLOT_LINK: &str = "/var/lib/thpkg/active-slot";
const BOOTED_OK_PATH: &str = "/var/lib/thpkg/booted-ok";

#[derive(Parser)]
#[command(
    name = "thpkg",
    about = "THISCLOUD package manager — A/B slot management",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Update to a new OS image (download, verify, write inactive slot, reboot)
    OsUpdate {
        /// Manifest URL or local path
        #[arg(short, long)]
        manifest: Option<String>,
        /// Skip reboot after writing slot
        #[arg(long)]
        no_reboot: bool,
    },
    /// Show current slot status, version, and health
    Status,
    /// Verify signature and hashes of a slot
    Verify {
        /// Slot to verify (default: active)
        #[arg(short, long)]
        slot: Option<String>,
    },
    /// Healthcheck hook — called by systemd after boot to mark slot as booted
    BootedOk,
    /// Initialize system (first-run: write config, run thiscloud init)
    Init {
        #[arg(short, long)]
        ip: String,
        #[arg(short, long)]
        role: String,
        #[arg(long)]
        cluster: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("thpkg=info")
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::OsUpdate { manifest, no_reboot } => {
            cmd_os_update(manifest.as_deref(), no_reboot).await
        }
        Commands::Status => cmd_status().await,
        Commands::Verify { slot } => cmd_verify(slot.as_deref()).await,
        Commands::BootedOk => cmd_booted_ok(),
        Commands::Init { ip, role, cluster } => cmd_init(&ip, &role, cluster.as_deref()),
    }
}

// ── os-update ──────────────────────────────────────────────────────────

async fn cmd_os_update(manifest_url: Option<&str>, no_reboot: bool) -> Result<()> {
    let url = manifest_url
        .unwrap_or("https://releases.thiscloud.io/manifest.json");

    info!("fetching manifest from {}", url);
    let manifest = slot::fetch_manifest(url).await?;
    info!("manifest version: {}", manifest.version);

    // Determine which slot is active, target the other
    let active = slot::read_active_slot()?;
    let target = slot::inactive_slot(&active);
    info!("active slot: {}, target: {}", active, target);

    // Download and write the target slot
    info!("downloading slot image...");
    let slot_path = slot::write_slot(&target, &manifest).await?;
    info!("slot written to {}", slot_path.display());

    // Verify the written slot
    info!("verifying slot...");
    verify::verify_slot(&slot_path, &manifest)?;

    // Set BootNext
    boot::set_boot_next(&target)?;
    info!("BootNext set to slot {}", target);

    // Mark as staged
    slot::mark_staged(&target, &manifest.version)?;

    if no_reboot {
        info!("slot staged — reboot manually to activate");
    } else {
        info!("rebooting in 5 seconds...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        Command::new("systemctl")
            .args(["reboot"])
            .status()
            .context("failed to reboot")?;
    }

    Ok(())
}

// ── status ─────────────────────────────────────────────────────────────

async fn cmd_status() -> Result<()> {
    let active = slot::read_active_slot()?;
    let version = slot::read_slot_version(&active)?;
    let booted_ok = Path::new(BOOTED_OK_PATH).exists();

    println!("Active slot: {}", active);
    println!("Version:     {}", version);
    println!("Booted OK:   {}", booted_ok);

    let inactive = slot::inactive_slot(&active);
    if let Ok(inv) = slot::read_slot_version(&inactive) {
        println!("Inactive:    {} (version {})", inactive, inv);
    }

    Ok(())
}

// ── verify ─────────────────────────────────────────────────────────────

async fn cmd_verify(slot_name: Option<&str>) -> Result<()> {
    let active = slot::read_active_slot()?;
    let name = slot_name.unwrap_or(&active);
    let slot_path = PathBuf::from(format!("/var/lib/thpkg/slots/{}", name));

    let manifest = slot::read_slot_manifest(&slot_path)?;
    verify::verify_slot(&slot_path, &manifest)?;

    println!("Slot {} verified successfully", name);
    Ok(())
}

// ── booted-ok ──────────────────────────────────────────────────────────

fn cmd_booted_ok() -> Result<()> {
    info!("running healthcheck...");
    boot::healthcheck()?;
    fs::write(BOOTED_OK_PATH, "ok")?;
    info!("booted-ok: healthcheck passed, slot marked as booted");
    Ok(())
}

// ── init ───────────────────────────────────────────────────────────────

fn cmd_init(ip: &str, role: &str, cluster: Option<&str>) -> Result<()> {
    info!("initializing THISCLOUD (ip={}, role={})", ip, role);

    let config_dir = Path::new("/etc/thiscloud");
    fs::create_dir_all(config_dir)?;

    let config = format!(
        r#"[node]
ip = "{ip}"
role = "{role}"
{cluster_section}
"#,
        cluster_section = cluster
            .map(|c| format!("cluster = \"{}\"", c))
            .unwrap_or_default(),
    );

    fs::write(config_dir.join("config.toml"), &config)?;
    info!("config written to /etc/thiscloud/config.toml");

    // Run thiscloud init if binary exists
    if Path::new("/usr/bin/thiscloud").exists() {
        std::process::Command::new("/usr/bin/thiscloud")
            .args(["init", "--ip", ip, "--role", role])
            .status()
            .context("failed to run thiscloud init")?;
    }

    Ok(())
}
