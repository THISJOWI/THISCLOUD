pub fn run_join(master: &str, ip: Option<&str>) -> anyhow::Result<()> {
    let ip = ip.unwrap_or("127.0.0.1");

    println!("Joining THISCLOUD cluster at {}...", master);
    println!("  Local IP: {}", ip);

    // TODO: Contact master node and register this node in the cluster
    println!("  (Cluster join not yet implemented - daemon registration pending)");

    Ok(())
}
