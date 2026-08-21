use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::{ACTIVE_SLOT_LINK, SLOTS_DIR};

/// Manifest downloaded from the release server describing a slot image.
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    /// Semantic version of this slot image.
    pub version: String,
    /// URL or relative path to the squashfs rootfs image.
    pub image_url: String,
    /// SHA-256 hex digest of the squashfs image.
    pub image_sha256: String,
    /// URL or relative path to the kernel.
    pub kernel_url: String,
    /// SHA-256 hex digest of the kernel.
    pub kernel_sha256: String,
    /// URL or relative path to the initrd.
    pub initrd_url: String,
    /// SHA-256 hex digest of the initrd.
    pub initrd_sha256: String,
    /// Ed25519 signature over the manifest fields (hex-encoded).
    pub signature: String,
    /// Version of the EL sysext baked into this slot.
    pub el_layer_version: String,
    /// Optional: additional sysext images bundled in this slot.
    #[serde(default)]
    pub sysexts: Vec<SysextEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SysextEntry {
    pub name: String,
    pub url: String,
    pub sha256: String,
}

/// Fetch a manifest from a URL or local path.
pub async fn fetch_manifest(url: &str) -> Result<Manifest> {
    if url.starts_with("http://") || url.starts_with("https://") {
        let body = reqwest::get(url)
            .await
            .context("failed to fetch manifest")?
            .text()
            .await
            .context("failed to read manifest body")?;
        serde_json::from_str(&body).context("failed to parse manifest JSON")
    } else {
        let data = fs::read_to_string(url).context("failed to read local manifest")?;
        serde_json::from_str(&data).context("failed to parse manifest JSON")
    }
}

/// Read which slot is currently active.
/// Returns "a" or "b".
pub fn read_active_slot() -> Result<String> {
    let link = Path::new(ACTIVE_SLOT_LINK);
    if !link.exists() {
        // First boot: check which slot exists
        let slot_a = PathBuf::from(format!("{}/a", SLOTS_DIR));
        if slot_a.exists() {
            return Ok("a".to_string());
        }
        let slot_b = PathBuf::from(format!("{}/b", SLOTS_DIR));
        if slot_b.exists() {
            return Ok("b".to_string());
        }
        bail!("no slots found — system not initialized");
    }

    fs::read_link(link)
        .map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .context("failed to read active slot link")?
        .context("active slot link has no filename")
}

/// Return the inactive slot name ("a" → "b", "b" → "a").
pub fn inactive_slot(active: &str) -> String {
    match active {
        "a" => "b".to_string(),
        "b" => "a".to_string(),
        _ => unreachable!("invalid slot name: {}", active),
    }
}

/// Read the version string stored in a slot directory.
pub fn read_slot_version(slot_name: &str) -> Result<String> {
    let version_file = PathBuf::from(format!("{}/{}/version", SLOTS_DIR, slot_name));
    fs::read_to_string(&version_file)
        .with_context(|| format!("failed to read version for slot {}", slot_name))
        .map(|s| s.trim().to_string())
}

/// Read the manifest stored inside a slot directory.
pub fn read_slot_manifest(slot_path: &Path) -> Result<Manifest> {
    let manifest_path = slot_path.join("manifest.json");
    let data = fs::read_to_string(&manifest_path)
        .with_context(|| format!("no manifest in slot {}", slot_path.display()))?;
    serde_json::from_str(&data).context("failed to parse slot manifest")
}

/// Download and write an entire slot (kernel, initrd, rootfs).
/// Returns the path to the written slot directory.
pub async fn write_slot(slot_name: &str, manifest: &Manifest) -> Result<PathBuf> {
    let slot_dir = PathBuf::from(format!("{}/{}", SLOTS_DIR, slot_name));
    fs::create_dir_all(&slot_dir)?;

    let base_url = manifest
        .image_url
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or("");

    // Download and verify kernel
    let kernel_url = resolve_url(base_url, &manifest.kernel_url);
    let kernel_path = slot_dir.join("vmlinuz");
    download_and_verify(&kernel_url, &kernel_path, &manifest.kernel_sha256, "kernel").await?;

    // Download and verify initrd
    let initrd_url = resolve_url(base_url, &manifest.initrd_url);
    let initrd_path = slot_dir.join("initrd");
    download_and_verify(&initrd_url, &initrd_path, &manifest.initrd_sha256, "initrd").await?;

    // Download and verify rootfs
    let rootfs_url = resolve_url(base_url, &manifest.image_url);
    let rootfs_path = slot_dir.join("rootfs.squashfs");
    download_and_verify(&rootfs_url, &rootfs_path, &manifest.image_sha256, "rootfs").await?;

    // Download optional sysexts
    for ext in &manifest.sysexts {
        let url = resolve_url(base_url, &ext.url);
        let path = slot_dir.join(format!("{}.squashfs", ext.name));
        download_and_verify(&url, &path, &ext.sha256, &ext.name).await?;
    }

    // Write manifest and version into the slot
    let manifest_json = serde_json::to_string_pretty(manifest)?;
    fs::write(slot_dir.join("manifest.json"), manifest_json)?;
    fs::write(slot_dir.join("version"), &manifest.version)?;

    Ok(slot_dir)
}

/// Mark a slot as staged (downloaded and ready, but not yet booted).
pub fn mark_staged(slot_name: &str, version: &str) -> Result<()> {
    let staged_path = PathBuf::from(format!("{}/{}/staged", SLOTS_DIR, slot_name));
    fs::write(&staged_path, version)?;
    Ok(())
}

// ── helpers ────────────────────────────────────────────────────────────

fn resolve_url(base: &str, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("{}/{}", base, url)
    }
}

async fn download_and_verify(
    url: &str,
    dest: &Path,
    expected_sha256: &str,
    label: &str,
) -> Result<()> {
    info!("downloading {} from {}", label, url);

    if dest.exists() {
        // Verify existing file
        let hash = file_sha256(dest)?;
        if hash == expected_sha256 {
            info!("{} already present and verified", label);
            return Ok(());
        }
        info!("{} hash mismatch, re-downloading", label);
    }

    let response = reqwest::get(url)
        .await
        .with_context(|| format!("failed to download {}", label))?
        .bytes()
        .await
        .with_context(|| format!("failed to read {} body", label))?;

    // Verify hash before writing
    let mut hasher = Sha256::new();
    hasher.update(&response);
    let hash = hex::encode(hasher.finalize());

    if hash != expected_sha256 {
        bail!(
            "{} hash mismatch: expected {}, got {}",
            label,
            expected_sha256,
            hash
        );
    }

    fs::write(dest, &response)
        .with_context(|| format!("failed to write {}", dest.display()))?;

    info!("{} verified and written (sha256: {})", label, &hash[..16]);
    Ok(())
}

fn file_sha256(path: &Path) -> Result<String> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}
