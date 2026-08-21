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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::{Manifest, SysextEntry};
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn make_test_slot(dir: &Path) -> (Manifest, PathBuf) {
        let slot_dir = dir.join("slot");
        fs::create_dir_all(&slot_dir).unwrap();

        let kernel = b"test-kernel-data";
        let initrd = b"test-initrd-data";
        let rootfs = b"test-rootfs-data";

        fs::write(slot_dir.join("vmlinuz"), kernel).unwrap();
        fs::write(slot_dir.join("initrd"), initrd).unwrap();
        fs::write(slot_dir.join("rootfs.squashfs"), rootfs).unwrap();

        let hash = |data: &[u8]| -> String {
            use sha2::Digest;
            let mut h = Sha256::new();
            h.update(data);
            hex::encode(h.finalize())
        };

        let manifest = Manifest {
            version: "0.4.0".to_string(),
            image_url: "rootfs.squashfs".to_string(),
            image_sha256: hash(rootfs),
            kernel_url: "vmlinuz".to_string(),
            kernel_sha256: hash(kernel),
            initrd_url: "initrd".to_string(),
            initrd_sha256: hash(initrd),
            signature: String::new(),
            el_layer_version: "el1.0".to_string(),
            sysexts: vec![],
        };

        (manifest, slot_dir)
    }

    #[test]
    fn test_verify_slot_passes() {
        let dir = tempdir().unwrap();
        let (manifest, slot_dir) = make_test_slot(dir.path());
        assert!(verify_slot(&slot_dir, &manifest).is_ok());
    }

    #[test]
    fn test_verify_slot_fails_on_tamper() {
        let dir = tempdir().unwrap();
        let (manifest, slot_dir) = make_test_slot(dir.path());

        // Tamper with kernel
        fs::write(slot_dir.join("vmlinuz"), b"tampered").unwrap();
        let result = verify_slot(&slot_dir, &manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("hash mismatch"));
    }

    #[test]
    fn test_verify_slot_fails_on_missing_file() {
        let dir = tempdir().unwrap();
        let slot_dir = dir.path().join("slot");
        fs::create_dir_all(&slot_dir).unwrap();

        let manifest = Manifest {
            version: "0.4.0".to_string(),
            image_url: "rootfs.squashfs".to_string(),
            image_sha256: "nope".to_string(),
            kernel_url: "vmlinuz".to_string(),
            kernel_sha256: "nope".to_string(),
            initrd_url: "initrd".to_string(),
            initrd_sha256: "nope".to_string(),
            signature: String::new(),
            el_layer_version: "el1.0".to_string(),
            sysexts: vec![],
        };
        assert!(verify_slot(&slot_dir, &manifest).is_err());
    }

    #[test]
    fn test_verify_slot_with_sysexts() {
        let dir = tempdir().unwrap();
        let (mut manifest, slot_dir) = make_test_slot(dir.path());

        let el_data = b"el-layer-data";
        fs::write(slot_dir.join("el-layer.squashfs"), el_data).unwrap();

        use sha2::Digest;
        let mut h = Sha256::new();
        h.update(el_data);
        manifest.sysexts.push(SysextEntry {
            name: "el-layer".to_string(),
            url: "el-layer.squashfs".to_string(),
            sha256: hex::encode(h.finalize()),
        });

        assert!(verify_slot(&slot_dir, &manifest).is_ok());
    }
}
