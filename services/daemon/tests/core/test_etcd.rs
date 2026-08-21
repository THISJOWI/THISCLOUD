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
async fn test_etcd_put_get() {
    let mut child = start_etcd(23791, "/tmp/thiscloud-etcd-test-1");
    let endpoint = "http://127.0.0.1:23791";

    if !wait_ready(endpoint).await {
        let _ = child.kill();
        let _ = child.wait();
        eprintln!("etcd not ready, skipping test");
        return;
    }

    let result = thiscloudd::core::etcd::EtcdClient::connect(endpoint).await;
    assert!(result.is_ok(), "connect failed: {:?}", result.err());

    let client = result.unwrap();
    client.put("/thiscloud/test/key", "value1").await.unwrap();
    client.put("/thiscloud/test/key2", "value2").await.unwrap();

    let v = client.get("/thiscloud/test/key").await.unwrap();
    assert_eq!(v.as_deref(), Some("value1"));

    let v2 = client.get("/thiscloud/test/key2").await.unwrap();
    assert_eq!(v2.as_deref(), Some("value2"));

    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test]
async fn test_etcd_get_missing() {
    let mut child = start_etcd(23792, "/tmp/thiscloud-etcd-test-2");
    let endpoint = "http://127.0.0.1:23792";

    if !wait_ready(endpoint).await {
        let _ = child.kill();
        let _ = child.wait();
        eprintln!("etcd not ready, skipping test");
        return;
    }

    let client = thiscloudd::core::etcd::EtcdClient::connect(endpoint)
        .await
        .unwrap();
    let v = client.get("/thiscloud/missing").await.unwrap();
    assert_eq!(v, None);

    let _ = child.kill();
    let _ = child.wait();
}

#[tokio::test]
async fn test_etcd_delete() {
    let mut child = start_etcd(23793, "/tmp/thiscloud-etcd-test-3");
    let endpoint = "http://127.0.0.1:23793";

    if !wait_ready(endpoint).await {
        let _ = child.kill();
        let _ = child.wait();
        eprintln!("etcd not ready, skipping test");
        return;
    }

    let client = thiscloudd::core::etcd::EtcdClient::connect(endpoint)
        .await
        .unwrap();
    client.put("/thiscloud/del/key", "value").await.unwrap();

    let v = client.get("/thiscloud/del/key").await.unwrap();
    assert_eq!(v.as_deref(), Some("value"));

    client.delete("/thiscloud/del/key").await.unwrap();

    let v = client.get("/thiscloud/del/key").await.unwrap();
    assert_eq!(v, None);

    let _ = child.kill();
    let _ = child.wait();
}
