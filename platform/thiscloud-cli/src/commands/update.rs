use super::api_client;
use semver::Version;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const DEFAULT_OWNER: &str = "THISJOWI";
const DEFAULT_REPO: &str = "THISCLOUD";
const VERSION_FILE: &str = "/etc/thiscloud/version";
const CONFIG_DIR: &str = "/etc/thiscloud";
const MANIFEST_NAME: &str = "manifest.json";

const SERVICES: &[&str] = &["thiscloudd", "thiscloud-api", "thiscloud-webui"];

#[derive(Debug, serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, serde::Deserialize)]
struct Manifest {
    version: String,
    assets: Vec<ManifestAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct ManifestAsset {
    name: String,
    sha256: String,
}

pub async fn run_update(check: bool, print_version: bool) -> anyhow::Result<()> {
    if print_version {
        match current_version()? {
            Some(v) => println!("{}", v),
            None => println!("unknown ({} missing)", VERSION_FILE),
        }
        return Ok(());
    }

    let (owner, repo) = repo();
    let token = std::env::var("THISCLOUD_UPDATE_TOKEN").ok();
    let client = api_client();

    if !check && !is_root() {
        // Non-root: report availability without installing.
        let latest = fetch_latest_release(&client, &owner, &repo, token.as_deref()).await?;
        let installed = current_version()?.unwrap_or_else(|| Version::new(0, 0, 0));
        println!(
            "Update available: {} (installed: {})",
            latest.tag_name, installed
        );
        println!("Run with sudo to install: sudo thiscloud update");
        return Ok(());
    }

    run_update_inner(&client, &owner, &repo, token.as_deref(), check).await
}

async fn run_update_inner(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    token: Option<&str>,
    check: bool,
) -> anyhow::Result<()> {
    let installed = current_version()?;

    let latest = fetch_latest_release(client, owner, repo, token).await?;
    let latest_ver = version_from_tag(&latest.tag_name)
        .ok_or_else(|| anyhow::anyhow!("cannot parse release tag as semver: {}", latest.tag_name))?;

    match &installed {
        Some(inst) if inst >= &latest_ver => {
            println!("THISCLOUD is up to date ({})", inst);
            return Ok(());
        }
        _ => {}
    }

    println!(
        "Update available: {} (installed: {})",
        latest.tag_name,
        installed.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "unknown".into())
    );
    if let Some(body) = &latest.body {
        if !body.trim().is_empty() {
            println!("\nRelease notes:\n{}", body.trim());
        }
    }

    if check {
        println!("\nRun `sudo thiscloud update` to install.");
        return Ok(());
    }

    println!("\n==> Downloading release assets");
    let work_dir = std::env::temp_dir().join(format!("thiscloud-update-{}", std::process::id()));
    fs::create_dir_all(&work_dir)?;
    let result = install_release(&latest, &installed, &work_dir).await;

    let _ = fs::remove_dir_all(&work_dir);
    result
}

async fn install_release(
    release: &GithubRelease,
    installed: &Option<Version>,
    work_dir: &Path,
) -> anyhow::Result<()> {
    // 1. Download manifest.json and verify against the release asset list.
    let manifest_asset = release
        .assets
        .iter()
        .find(|a| a.name == MANIFEST_NAME)
        .ok_or_else(|| anyhow::anyhow!("release has no {} — refusing to install", MANIFEST_NAME))?;
    let manifest_path = work_dir.join(MANIFEST_NAME);
    download_file(&manifest_asset.browser_download_url, &manifest_path, None).await?;

    let manifest: Manifest = {
        let raw = fs::read_to_string(&manifest_path)?;
        serde_json::from_str(&raw)?
    };

    // 2. Download every asset listed in the manifest.
    for asset in &manifest.assets {
        let url = release
            .assets
            .iter()
            .find(|a| a.name == asset.name)
            .map(|a| a.browser_download_url.clone())
            .ok_or_else(|| {
                anyhow::anyhow!("asset {} in manifest not found in release", asset.name)
            })?;
        download_file(&url, &work_dir.join(&asset.name), None).await?;
    }

    // 3. Verify integrity before touching the system.
    verify_manifest(work_dir, &manifest)?;

    // 4. Backup current state.
    let backup_dir = backup_current(installed)?;
    println!("Backup: {}", backup_dir.display());

    // 5. Install.
    if let Err(e) = install_artifacts(work_dir, &manifest.version) {
        eprintln!("Install failed: {}", e);
        rollback(&backup_dir);
        return Err(e);
    }

    // 6. Restart services.
    let mut failed = Vec::new();
    for svc in SERVICES {
        if !restart_service(svc) {
            failed.push(*svc);
        }
    }

    if !failed.is_empty() {
        let msg = format!("services failed to restart: {}", failed.join(", "));
        eprintln!("{}", msg);
        rollback(&backup_dir);
        return Err(anyhow::anyhow!(msg));
    }

    // 7. Record installed version.
    fs::write(VERSION_FILE, format!("{}\n", manifest.version))?;
    println!("==> Updated to {}", manifest.version);
    Ok(())
}

fn current_version() -> anyhow::Result<Option<Version>> {
    match fs::read_to_string(VERSION_FILE) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            match Version::parse(trimmed) {
                Ok(v) => Ok(Some(v)),
                Err(_) => {
                    eprintln!(
                        "warning: {} contains invalid semver {:?}; treating as unknown",
                        VERSION_FILE, trimmed
                    );
                    Ok(None)
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn repo() -> (String, String) {
    match std::env::var("THISCLOUD_UPDATE_REPO") {
        Ok(val) => {
            let trimmed = val.trim().trim_start_matches("https://github.com/");
            let parts: Vec<&str> = trimmed.split('/').collect();
            if parts.len() >= 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (DEFAULT_OWNER.to_string(), DEFAULT_REPO.to_string())
            }
        }
        Err(_) => (DEFAULT_OWNER.to_string(), DEFAULT_REPO.to_string()),
    }
}

fn version_from_tag(tag: &str) -> Option<Version> {
    let stripped = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(stripped).ok()
}

async fn fetch_latest_release(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    token: Option<&str>,
) -> anyhow::Result<GithubRelease> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    );
    let mut req = client.get(&url).header("User-Agent", "thiscloud-update/0.1");
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await.map_err(|e| {
        anyhow::anyhow!(
            "cannot reach GitHub API ({url}): {e}\n  Check network or set THISCLOUD_UPDATE_TOKEN"
        )
    })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "GitHub API error {} for {}/{}: {}",
            status,
            owner,
            repo,
            body
        ));
    }
    Ok(resp.json().await?)
}

async fn download_file(
    url: &str,
    dest: &Path,
    token: Option<&str>,
) -> anyhow::Result<()> {
    // Asset downloads (web-ui tarball, RPMs) can exceed the default 30s
    // request timeout — use a dedicated client with a longer timeout.
    let dl_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .expect("failed to build download client");
    let mut req = dl_client.get(url).header("User-Agent", "thiscloud-update/0.1");
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await.map_err(|e| {
        anyhow::anyhow!("download failed for {url}: {e}")
    })?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "download failed for {url}: HTTP {}",
            resp.status()
        ));
    }
    let bytes = resp.bytes().await?;
    fs::write(dest, &bytes)?;
    Ok(())
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 65536];
    use std::io::Read;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn verify_manifest(work_dir: &Path, manifest: &Manifest) -> anyhow::Result<()> {
    for asset in &manifest.assets {
        let path = work_dir.join(&asset.name);
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "asset {} missing after download — aborting",
                asset.name
            ));
        }
        let actual = sha256_file(&path)?;
        if actual != asset.sha256 {
            return Err(anyhow::anyhow!(
                "checksum mismatch for {}: expected {} got {} — aborting, no changes made",
                asset.name,
                asset.sha256,
                actual
            ));
        }
    }
    Ok(())
}

fn is_root() -> bool {
    let out = Command::new("id").arg("-u").output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim() == "0",
        Err(_) => false,
    }
}

fn backup_current(installed: &Option<Version>) -> anyhow::Result<PathBuf> {
    let suffix = installed
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "unknown".into());
    let backup_dir = PathBuf::from(CONFIG_DIR).join(format!("backup-v{}", suffix));
    fs::create_dir_all(&backup_dir)?;

    // Config files (never overwritten, but kept for rollback).
    if let Ok(entries) = fs::read_dir(CONFIG_DIR) {
        for e in entries.flatten() {
            let name = e.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str.starts_with("backup-v") {
                continue;
            }
            let src = e.path();
            if src.is_file() {
                let _ = fs::copy(&src, backup_dir.join(name));
            }
        }
    }

    // Directly-replaced binaries.
    for (src, name) in [
        ("/usr/local/bin/thiscloud-api", "thiscloud-api"),
        ("/usr/sbin/thiscloudd", "thiscloudd"),
        ("/usr/bin/thiscloud", "thiscloud"),
    ] {
        if let Ok(meta) = fs::metadata(src) {
            if meta.is_file() {
                let _ = fs::copy(src, backup_dir.join(name));
            }
        }
    }

    // Web UI tree.
    let webui_src = Path::new("/usr/share/thiscloud/web-ui");
    if webui_src.exists() {
        let dst = backup_dir.join("web-ui");
        let _ = fs::create_dir_all(&dst);
        copy_tree(webui_src, &dst);
    }

    // systemd units.
    let sys_src = Path::new("/etc/systemd/system");
    if let Ok(entries) = fs::read_dir(sys_src) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("thiscloud") && name.ends_with(".service") {
                let _ = fs::copy(e.path(), backup_dir.join(name));
            }
        }
    }

    // RPM package versions (for dnf downgrade on rollback).
    if let Ok(o) = Command::new("rpm").args(["-q", "thiscloudd", "thiscloud"]).output() {
        if o.status.success() {
            let _ = fs::write(backup_dir.join("rpm-versions.txt"), o.stdout);
        }
    }

    Ok(backup_dir)
}

fn copy_tree(src: &Path, dst: &Path) {
    if let Ok(entries) = fs::read_dir(src) {
        for e in entries.flatten() {
            let path = e.path();
            let target = dst.join(e.file_name());
            if path.is_dir() {
                let _ = fs::create_dir_all(&target);
                copy_tree(&path, &target);
            } else {
                let _ = fs::copy(&path, &target);
            }
        }
    }
}

fn install_artifacts(work_dir: &Path, _version: &str) -> anyhow::Result<()> {
    // 1. RPMs.
    let rpms: Vec<PathBuf> = fs::read_dir(work_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "rpm").unwrap_or(false))
        .collect();
    if !rpms.is_empty() {
        println!("==> Installing RPM packages");
        let status = Command::new("dnf")
            .args(dnf_localinstall_args(&rpms))
            .status()
            .or_else(|_| Command::new("rpm").args(["-Uvh"]).args(rpms.iter().map(|p| p.as_os_str())).status())?;
        if !status.success() {
            return Err(anyhow::anyhow!("RPM install failed (status {})", status));
        }
    }

    // 2. thiscloud-api binary.
    let api_src = work_dir.join("thiscloud-api-linux-amd64");
    if api_src.exists() {
        println!("==> Installing thiscloud-api");
        install_binary(&api_src, Path::new("/usr/local/bin/thiscloud-api"))?;
    }

    // 3. Web UI tarball.
    let webui_tgz = work_dir.join("thiscloud-webui.tar.gz");
    if webui_tgz.exists() {
        println!("==> Installing web-ui");
        let dst = Path::new("/usr/share/thiscloud/web-ui");
        let _ = fs::remove_dir_all(dst);
        fs::create_dir_all(dst)?;
        extract_tar_gz(&webui_tgz, dst)?;
    }

    // 4. systemd units tarball.
    let sys_tgz = work_dir.join("thiscloud-systemd.tar.gz");
    if sys_tgz.exists() {
        println!("==> Installing systemd units");
        extract_tar_gz(&sys_tgz, Path::new("/etc/systemd/system"))?;
        let _ = Command::new("systemctl").arg("daemon-reload").status();
    }

    Ok(())
}

// dnf_localinstall_args builds the dnf command for installing local RPMs.
// Configured repositories are disabled (--disablerepo=*) because the install
// must not depend on external repo metadata: the ISO installer leaves a local
// repo pointing at file:///tmp/thiscloud-repo/thiscloud which vanishes after a
// reboot, and any stale/broken repo would make dnf abort before touching the
// local packages. Dependency resolution still works against the installed
// package database plus the local RPMs.
fn dnf_localinstall_args(rpms: &[PathBuf]) -> Vec<String> {
    let mut args = vec![
        "localinstall".to_string(),
        "-y".to_string(),
        "--disablerepo=*".to_string(),
    ];
    for p in rpms {
        args.push(p.to_str().unwrap_or_default().to_string());
    }
    args
}

fn make_executable(path: &str) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

// install_binary atomically replaces dest with src. Overwriting a running
// executable directly fails with ETXTBSY ("Text file busy"); staging a temp
// file in the same directory and renaming it over the target lets the running
// process keep its old inode.
fn install_binary(src: &Path, dest: &Path) -> anyhow::Result<()> {
    let tmp = dest.with_extension("tmp");
    fs::copy(src, &tmp)?;
    let tmp_str = tmp
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("invalid temp path: {}", tmp.display()))?;
    make_executable(tmp_str)?;
    fs::rename(&tmp, dest)?;
    Ok(())
}

fn extract_tar_gz(tarball: &Path, dest: &Path) -> anyhow::Result<()> {
    let mut cmd = Command::new("tar")
        .arg("-xzf")
        .arg(tarball)
        .arg("-C")
        .arg(dest)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()?;
    let status = cmd.wait()?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "tar extraction failed for {} (status {})",
            tarball.display(),
            status
        ));
    }
    Ok(())
}

fn restart_service(name: &str) -> bool {
    Command::new("systemctl")
        .args(["restart", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn rollback(backup_dir: &Path) {
    eprintln!("==> Rolling back from {}", backup_dir.display());

    // Restore directly-replaced binaries. Only thiscloud-api is a raw
    // binary install; thiscloudd and thiscloud-cli are RPM-managed, so
    // restoring them here would desync /usr/bin/thiscloud and
    // /usr/sbin/thiscloudd from the RPM database (the next update's
    // "already installed" no-op would never repair the stale file). The
    // RPMs (and the dnf downgrade below) own those paths.
    let api_src = backup_dir.join("thiscloud-api");
    if api_src.exists() {
        let _ = install_binary(&api_src, Path::new("/usr/local/bin/thiscloud-api"));
    }

    // Restore web-ui tree.
    let webui_src = backup_dir.join("web-ui");
    if webui_src.exists() {
        let dst = Path::new("/usr/share/thiscloud/web-ui");
        let _ = fs::remove_dir_all(dst);
        let _ = fs::create_dir_all(dst);
        copy_tree(&webui_src, dst);
    }

    // Restore systemd units.
    if let Ok(entries) = fs::read_dir(backup_dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("thiscloud") && name.ends_with(".service") {
                let _ = fs::copy(e.path(), Path::new("/etc/systemd/system").join(name));
            }
        }
    }

    let _ = Command::new("systemctl").arg("daemon-reload").status();
    for svc in SERVICES {
        let _ = restart_service(svc);
    }

    // Downgrade RPM-managed packages if they were upgraded.
    let rpm_ver = backup_dir.join("rpm-versions.txt");
    if rpm_ver.exists() {
        if let Ok(raw) = fs::read_to_string(&rpm_ver) {
            let specs: Vec<&str> = raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
            if !specs.is_empty() {
                let _ = Command::new("dnf")
                    .arg("downgrade")
                    .arg("-y")
                    .args(&specs)
                    .status();
            }
        }
    }

    eprintln!("Rollback complete. Review service status: systemctl status thiscloudd thiscloud-api thiscloud-webui");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dnf_localinstall_disables_configured_repos() {
        let rpms = vec![
            PathBuf::from("/tmp/thiscloudd-1.2.3.x86_64.rpm"),
            PathBuf::from("/tmp/thiscloud-1.2.3.x86_64.rpm"),
        ];
        let args = dnf_localinstall_args(&rpms);
        assert_eq!(args[0], "localinstall");
        assert_eq!(args[1], "-y");
        // Stale/absent repo metadata (e.g. the ISO installer's
        // file:///tmp/thiscloud-repo/thiscloud) must not abort the install.
        assert!(args.contains(&"--disablerepo=*".to_string()));
        assert_eq!(args.last().unwrap(), "/tmp/thiscloud-1.2.3.x86_64.rpm");
        assert!(args.iter().all(|a| !a.contains('\n')));
    }

    #[test]
    fn parses_v_prefixed_tags() {
        let v = version_from_tag("v0.2.0").unwrap();
        assert_eq!(v, Version::new(0, 2, 0));
        let v = version_from_tag("0.2.0").unwrap();
        assert_eq!(v, Version::new(0, 2, 0));
        assert!(version_from_tag("not-a-tag").is_none());
    }

    #[test]
    fn semver_ordering_used_for_compare() {
        let a = Version::parse("0.2.0").unwrap();
        let b = Version::parse("0.10.0").unwrap();
        assert!(b > a);
    }

    #[test]
    fn resolves_repo_env() {
        std::env::set_var("THISCLOUD_UPDATE_REPO", "owner/mycloud");
        let (o, r) = repo();
        assert_eq!(o, "owner");
        assert_eq!(r, "mycloud");
        std::env::remove_var("THISCLOUD_UPDATE_REPO");
    }

    #[test]
    fn sha256_file_matches_known_digest() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f");
        fs::write(&p, b"hello").unwrap();
        assert_eq!(
            sha256_file(&p).unwrap(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn verify_manifest_rejects_bad_checksum() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.rpm"), b"corrupt").unwrap();
        let m = Manifest {
            version: "0.2.0".into(),
            assets: vec![ManifestAsset {
                name: "a.rpm".into(),
                sha256: "0".repeat(64),
            }],
        };
        assert!(verify_manifest(dir.path(), &m).is_err());
    }

    #[test]
    fn verify_manifest_accepts_good_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let content = b"package-data";
        fs::write(dir.path().join("a.rpm"), content).unwrap();
        let digest = sha256_file(&dir.path().join("a.rpm")).unwrap();
        let m = Manifest {
            version: "0.2.0".into(),
            assets: vec![ManifestAsset {
                name: "a.rpm".into(),
                sha256: digest,
            }],
        };
        assert!(verify_manifest(dir.path(), &m).is_ok());
    }

    #[test]
    fn install_binary_replaces_target_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("new-binary");
        let dest = dir.path().join("thiscloud-api");
        fs::write(&src, b"new-content").unwrap();
        fs::write(&dest, b"old-content").unwrap();

        install_binary(&src, &dest).unwrap();

        use std::io::Read;
        let mut buf = Vec::new();
        fs::File::open(&dest).unwrap().read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"new-content");
        assert!(!dest.with_extension("tmp").exists());
    }
}