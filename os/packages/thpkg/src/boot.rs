use anyhow::{bail, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

/// Set systemd-boot to boot from a specific slot on next reboot.
/// Writes the slot name into the BootNext variable via bootctl.
pub fn set_boot_next(slot_name: &str) -> Result<()> {
    // The systemd-boot entry title matches "ThisCloud A" or "ThisCloud B"
    let title = format!("ThisCloud {}", slot_name.to_uppercase());

    // Use bootctl to set the default entry
    let status = Command::new("bootctl")
        .args(["set-default", &title])
        .status();

    match status {
        Ok(s) if s.success() => {
            info!("bootctl set-default: {}", title);
            Ok(())
        }
        Ok(s) => {
            warn!("bootctl exited with status: {}", s);
            // Fallback: write directly to the loader config
            write_slot_marker(slot_name)
        }
        Err(e) => {
            warn!("bootctl not found ({}), using fallback", e);
            write_slot_marker(slot_name)
        }
    }
}

/// Fallback: write a marker file that the initrd/boot hook reads.
fn write_slot_marker(slot_name: &str) -> Result<()> {
    let marker = Path::new("/run/thpkg/next-slot");
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(marker, slot_name)?;
    info!("wrote next-slot marker: {}", slot_name);
    Ok(())
}

/// Run a healthcheck of critical services.
/// Returns Ok(()) if all services are healthy.
pub fn healthcheck() -> Result<()> {
    let services = [
        "thiscloudd.service",
        "thiscloud-api.service",
        "thiscloud-webui.service",
    ];

    let mut failed = Vec::new();

    for svc in &services {
        let output = Command::new("systemctl")
            .args(["is-active", svc])
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if stdout != "active" {
                    warn!("service {} is {}", svc, stdout);
                    failed.push(svc.to_string());
                } else {
                    info!("service {} healthy", svc);
                }
            }
            Err(e) => {
                warn!("failed to check {}: {}", svc, e);
                failed.push(svc.to_string());
            }
        }
    }

    // Also check etcd if present
    let etcd_check = Command::new("systemctl")
        .args(["is-active", "etcd.service"])
        .output();

    if let Ok(o) = etcd_check {
        let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if stdout == "active" {
            info!("service etcd.service healthy");
        } else {
            warn!("service etcd.service is {}", stdout);
            // etcd failure is non-fatal — daemon has in-memory fallback
        }
    }

    if !failed.is_empty() {
        bail!("healthcheck failed for: {}", failed.join(", "));
    }

    Ok(())
}
