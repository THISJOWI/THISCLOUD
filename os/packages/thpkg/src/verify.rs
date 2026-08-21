use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tracing::info;

use crate::slot::Manifest;

/// Verify all files in a slot against the manifest hashes.
/// Currently verifies SHA-256 only; Ed25519 signature verification
/// is a TODO (requires public key distribution).
pub fn verify_slot(slot_path: &Path, manifest: &Manifest) -> Result<()> {
    // Verify kernel
    let kernel_path = slot_path.join("vmlinuz");
    verify_file(&kernel_path, &manifest.kernel_sha256, "kernel")?;

    // Verify initrd
    let initrd_path = slot_path.join("initrd");
    verify_file(&initrd_path, &manifest.initrd_sha256, "initrd")?;

    // Verify rootfs
    let rootfs_path = slot_path.join("rootfs.squashfs");
    verify_file(&rootfs_path, &manifest.image_sha256, "rootfs")?;

    // Verify optional sysexts
    for ext in &manifest.sysexts {
        let path = slot_path.join(format!("{}.squashfs", ext.name));
        verify_file(&path, &ext.sha256, &ext.name)?;
    }

    // TODO: Ed25519 signature verification over the manifest
    // Requires embedding the signing public key in the image or
    // verifying against a known-good key during os-update.
    // For now, sha256 integrity is sufficient for the prototype.

    info!("all slot files verified");
    Ok(())
}

fn verify_file(path: &Path, expected_sha256: &str, label: &str) -> Result<()> {
    if !path.exists() {
        bail!("{} missing: {}", label, path.display());
    }

    let data = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hex::encode(hasher.finalize());

    if hash != expected_sha256 {
        bail!(
            "{} hash mismatch at {}: expected {}, got {}",
            label,
            path.display(),
            expected_sha256,
            hash
        );
    }

    info!("{} verified (sha256: {}…)", label, &hash[..16]);
    Ok(())
}
