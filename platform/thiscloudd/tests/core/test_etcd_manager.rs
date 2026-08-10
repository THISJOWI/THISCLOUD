fn config(port: u16, data_dir: &str) -> thiscloudd::config::EtcdConfig {
    thiscloudd::config::EtcdConfig {
        embedded: true,
        port,
        peer_port: port + 1,
        data_dir: data_dir.to_string(),
        quota_backend: "1GB".to_string(),
    }
}

#[tokio::test]
async fn test_etcd_manager_start_connect() {
    // EtcdManager.start() spawns the etcd process itself on config.port.
    let mut manager =
        thiscloudd::core::EtcdManager::new(config(23801, "/tmp/thiscloud-etcd-mgr-1"));
    manager.start().await.unwrap();

    assert!(manager.is_running());

    let client = manager.connect().await.expect("etcd should be ready");
    client.put("/thiscloud/mgr/test", "ok").await.unwrap();
    let v = client.get("/thiscloud/mgr/test").await.unwrap();
    assert_eq!(v.as_deref(), Some("ok"));

    manager.stop();
    assert!(!manager.is_running());
}

#[tokio::test]
async fn test_etcd_manager_disabled() {
    let mut cfg = config(23802, "/tmp/thiscloud-etcd-mgr-2");
    cfg.embedded = false;

    let mut manager = thiscloudd::core::EtcdManager::new(cfg);
    assert!(!manager.enabled());
    manager.start().await.unwrap();
    assert!(!manager.is_running());
    manager.stop();
}
