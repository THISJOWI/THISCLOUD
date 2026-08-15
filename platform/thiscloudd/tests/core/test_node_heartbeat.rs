use thiscloudd::node::heartbeat::SelfHeartbeat;
use thiscloudd::node::model::{Node, NodeHeartbeat, NodeRole, NodeState};
use thiscloudd::node::{MemoryNodeStore, NodeModule};
use tokio::sync::Mutex;

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

async fn module_with_node(ttl: u64) -> (NodeModule, Node) {
    let mut module = NodeModule::new(Box::new(MemoryNodeStore::default()));
    let node = module
        .register(Node {
            id: "node-heartbeat-1".to_string(),
            name: "worker-1".to_string(),
            role: NodeRole::Worker,
            address: "127.0.0.1:8080".to_string(),
            hostname: "worker-1".to_string(),
            cpus_total: 4,
            cpus_used: 0,
            memory_total_mb: 8192,
            memory_used_mb: 0,
            vms: 0,
            state: NodeState::Online,
            last_seen_secs: now(),
            ttl_secs: ttl,
            labels: Vec::new(),
        })
        .await
        .unwrap();
    (module, node)
}

#[tokio::test]
async fn test_self_heartbeat_keeps_node_online_locally() {
    let (mut module, node) = module_with_node(1).await;

    // TTL (1s) expires -> node offline, nothing is heartbeating it.
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    assert_eq!(module.list().await.unwrap()[0].state, NodeState::Offline);

    // A local self-heartbeat revives it (no masters → local path).
    let mut hb = SelfHeartbeat::new(node, Vec::new());
    hb.beat(
        Some(&mut module),
        NodeHeartbeat {
            cpus_used: 1,
            memory_used_mb: 512,
            vms: 1,
        },
    )
    .await
    .unwrap();

    let node = module.list().await.unwrap().remove(0);
    assert_eq!(node.state, NodeState::Online);
    assert_eq!(node.cpus_used, 1);
}

#[tokio::test]
async fn test_self_heartbeat_posts_to_master() {
    use thiscloudd::node::http::{app as node_http_app, NodeApiState};
    use std::sync::Arc;

    // Server side: the master's own store + router.
    let master_module = Arc::new(Mutex::new(NodeModule::new(Box::new(MemoryNodeStore::default()))));
    {
        let mut m = master_module.lock().await;
        m.register(Node {
            id: "node-heartbeat-2".to_string(),
            name: "worker-2".to_string(),
            role: NodeRole::Worker,
            address: "127.0.0.1:8081".to_string(),
            hostname: "worker-2".to_string(),
            cpus_total: 4,
            cpus_used: 0,
            memory_total_mb: 8192,
            memory_used_mb: 0,
            vms: 0,
            state: NodeState::Online,
            last_seen_secs: now(),
            ttl_secs: 1,
            labels: Vec::new(),
        })
        .await
        .unwrap();
    }

    let router = axum::Router::new().nest(
        "/api/v1",
        node_http_app(NodeApiState::new(master_module.clone())),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Let the master's view of the worker expire.
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    assert_eq!(
        master_module.lock().await.list().await.unwrap()[0].state,
        NodeState::Offline
    );

    // Worker daemon heartbeats the master over HTTP. The worker's own store is
    // irrelevant to the remote path.
    let node = Node {
        id: "node-heartbeat-2".to_string(),
        name: "worker-2".to_string(),
        role: NodeRole::Worker,
        address: "127.0.0.1:8081".to_string(),
        hostname: "worker-2".to_string(),
        cpus_total: 4,
        cpus_used: 0,
        memory_total_mb: 8192,
        memory_used_mb: 0,
        vms: 0,
        state: NodeState::Online,
        last_seen_secs: now(),
        ttl_secs: 1,
        labels: Vec::new(),
    };
    let mut hb = SelfHeartbeat::new(node, vec![format!("http://{addr}")]);
    hb.beat(
        None,
        NodeHeartbeat {
            cpus_used: 0,
            memory_used_mb: 0,
            vms: 0,
        },
    )
    .await
    .unwrap();

    let node = master_module.lock().await.list().await.unwrap().remove(0);
    assert_eq!(node.state, NodeState::Online);
}