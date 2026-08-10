use super::api_client;
use std::path::PathBuf;

fn config_path() -> PathBuf {
    std::env::var("THISCLOUD_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/etc/thiscloud"))
        .join("config.toml")
}

fn daemon_url() -> String {
    std::env::var("THISCLOUD_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

pub async fn run_status() -> anyhow::Result<()> {
    println!("THISCLOUD Cluster Status");
    println!("========================");

    let pid_file = PathBuf::from("/var/run/thiscloudd.pid");
    if pid_file.exists() {
        let pid = std::fs::read_to_string(&pid_file)?;
        println!("Daemon: Running (PID: {})", pid.trim());
    } else {
        // Fallback: probe the daemon over HTTP. A response (even 404) means the
        // HTTP server is up; connection refused means it is down.
        let url = daemon_url();
        match api_client().get(&url).send().await {
            Ok(_) => println!("Daemon: Running ({})", url),
            Err(_) => println!("Daemon: Not running"),
        }
    }

    let config_path = config_path();
    if config_path.exists() {
        let config_content = std::fs::read_to_string(&config_path)?;
        let config: toml::Value = config_content.parse()?;

        println!(
            "\nCluster: {}",
            config["cluster"]["name"].as_str().unwrap_or("unknown")
        );

        if let Some(nodes) = config["cluster"]["nodes"].as_array() {
            println!("Nodes: {}", nodes.len());
            for (i, node) in nodes.iter().enumerate() {
                let ip = node["ip"].as_str().unwrap_or("unknown");
                let role = node["role"].as_str().unwrap_or("unknown");
                println!("  {}. {} ({})", i + 1, ip, role);
            }
        }
    } else {
        println!("\nNo configuration found. Run: thiscloud init");
    }

    Ok(())
}
