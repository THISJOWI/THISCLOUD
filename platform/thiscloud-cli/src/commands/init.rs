use std::fs;
use std::path::PathBuf;

fn config_root() -> PathBuf {
    std::env::var("THISCLOUD_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/etc/thiscloud"))
}

fn data_root() -> PathBuf {
    std::env::var("THISCLOUD_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/thiscloud"))
}

pub fn run_init(ip: Option<String>, role: &str) -> anyhow::Result<()> {
    let ip = ip.unwrap_or_else(|| "127.0.0.1".to_string());

    println!("Initializing THISCLOUD node...");
    println!("  IP: {}", ip);
    println!("  Role: {}", role);

    let config_dir = config_root();
    fs::create_dir_all(&config_dir)?;

    let config_content = format!(
        r#"[cluster]
name = "thiscloud-cluster"

[[cluster.nodes]]
ip = "{}"
role = "{}"

[node]
role = "{}"

[compute]
backend = "cloud-hypervisor"
http_bind = "127.0.0.1"
http_port = 8080

[network]
backend = "ovn"

[storage]
backend = "linstor"

[marketplace]
backend = "docker"
"#,
        ip, role, role
    );

    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, config_content)?;

    println!("Configuration created at: {}", config_path.display());

    let base = data_root();
    let data_dirs = vec![base.clone(), base.join("vms"), base.join("storage")];

    for dir in data_dirs {
        fs::create_dir_all(&dir)?;
        println!("Created directory: {}", dir.display());
    }

    println!("THISCLOUD node initialized successfully!");
    println!("\nNext steps:");
    println!(
        "  1. Edit {} to configure your cluster",
        config_path.display()
    );
    println!("  2. Start the daemon: systemctl start thiscloudd");
    println!("  3. Check status: thiscloud status");

    Ok(())
}
