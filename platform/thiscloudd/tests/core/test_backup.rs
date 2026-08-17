use std::process::{Child, Command};
use std::time::Duration;

fn start_etcd(port: u16, data_dir: &str) -> Child {
    Command::new("etcd")
        .arg("--name")
        .arg("test-etcd")
        .arg("--listen-client-urls")
        .arg(format!("http://127.0.0.1:{}", port))
        .arg("--advertise-client-urls")
        .arg(format!("http://127.0.0.1:{}", port))
        .arg("--data-dir")
        .arg(data_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start etcd")
}

async fn wait_ready(endpoint: &str) -> bool {
    for _ in 0..100 {
        if let Ok(mut c) = etcd_client::Client::connect([endpoint], None).await {
            if let Ok(resp) = c.status().await {
                if resp.header().is_some() {
                    return true;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test]
async fn test_backup_create_restore_roundtrip() {
    let mut child = start_etcd(23797, "/tmp/thiscloud-etcd-backup-1");
    let endpoint = "http://127.0.0.1:23797";
    if !wait_ready(endpoint).await {
        let _ = child.kill();
        let _ = child.wait();
        eprintln!("etcd not ready, skipping test");
        return;
    }

    let client = thiscloudd::core::EtcdClient::connect(endpoint).await.unwrap();

    // Seed some state across a few keys.
    client.put("/thiscloud/compute/vm-1", "{\"name\":\"web\"}").await.unwrap();
    client.put("/thiscloud/network/net-1", "{\"name\":\"mgmt\"}").await.unwrap();
    client.put("/thiscloud/node/master-1", "{\"role\":\"master\"}").await.unwrap();

    // Create a snapshot.
    let dir = std::env::temp_dir().join(format!("thiscloud-backup-test-{}", std::process::id()));
    let service = thiscloudd::backup::BackupService::new(Some(client.clone()), &dir, 5);
    let created = service.create_snapshot().await.expect("create snapshot");
    assert_eq!(created.entries, 3);
    assert!(created.name.starts_with("thiscloud-"));

    // Wipe state, then restore and verify all keys are back.
    client.wipe().await.expect("wipe");
    let empty = client.dump().await.unwrap();
    assert!(empty.is_empty(), "expected empty state after wipe");

    service.restore_snapshot(&created.name).await.expect("restore snapshot");

    let restored = client.dump().await.unwrap();
    assert_eq!(restored.len(), 3, "expected 3 keys restored");
    let mut keys: Vec<String> = restored.iter().map(|(k, _)| k.clone()).collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "/thiscloud/compute/vm-1",
            "/thiscloud/network/net-1",
            "/thiscloud/node/master-1"
        ]
    );

    // Snapshot survives in the listing.
    let listed = service.list_snapshots().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, created.name);

    let _ = std::fs::remove_dir_all(&dir);
    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test]
async fn test_backup_prune_keeps_retention() {
    // prune() only touches the filesystem, so it works without etcd.
    let dir = std::env::temp_dir().join(format!("thiscloud-backup-prune-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for i in 1..=5u64 {
        std::fs::write(
            dir.join(format!("thiscloud-17{}.json", i)),
            "{}",
        )
        .unwrap();
    }

    let service = thiscloudd::backup::BackupService::new(None, &dir, 2);
    let removed = service.prune().await.expect("prune");
    assert_eq!(removed.len(), 3, "expected 3 pruned, got {:?}", removed);
    let remaining = service.list_snapshots().unwrap();
    assert_eq!(remaining.len(), 2, "expected 2 remaining, got {:?}", remaining);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_backup_rejects_path_traversal() {
    let dir = std::env::temp_dir().join(format!("thiscloud-backup-traversal-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let service = thiscloudd::backup::BackupService::new(None, &dir, 1);

    for bad in ["../../etc/passwd", "thiscloud-x.json/../../x", "foo.json", "thiscloud"] {
        let result = service.restore_snapshot(bad).await;
        assert!(result.is_err(), "expected error for {}", bad);
    }

    let _ = std::fs::remove_dir_all(&dir);
}