use thiscloudd::node::model::{Node, NodeHeartbeat, NodeRole, NodeState};
use thiscloudd::node::{MemoryNodeStore, NodeModule};

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn sample_node(id: &str, cpus: u32, mem: u32, labels: Vec<String>, ttl: u64) -> Node {
    Node {
        id: id.to_string(),
        name: format!("node-{}", id),
        role: NodeRole::Worker,
        address: format!("10.0.0.{}:8080", id),
        hostname: format!("{}.thiscloud.local", id),
        cpus_total: cpus,
        cpus_used: 0,
        memory_total_mb: mem,
        memory_used_mb: 0,
        vms: 0,
        state: NodeState::Online,
        last_seen_secs: now(),
        ttl_secs: ttl,
        labels,
    }
}

fn new_module() -> NodeModule {
    NodeModule::new(Box::new(MemoryNodeStore::default()))
}

#[tokio::test]
async fn test_register_and_list_nodes() {
    let mut module = new_module();
    let node = module.register(sample_node("a", 4, 8192, Vec::new(), 30)).await.unwrap();
    assert!(!node.id.is_empty());

    let nodes = module.list().await.unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].name, "node-a");
    assert_eq!(nodes[0].state, NodeState::Online);
}

#[tokio::test]
async fn test_heartbeat_updates_usage_and_liveness() {
    let mut module = new_module();
    let node = module.register(sample_node("b", 4, 8192, Vec::new(), 1)).await.unwrap();
    let id = node.id.clone();

    // Let the 1s TTL expire -> node goes offline, then heartbeat revives it.
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    let nodes = module.list().await.unwrap();
    assert_eq!(nodes[0].state, NodeState::Offline);

    let refreshed = module
        .heartbeat(
            &id,
            NodeHeartbeat {
                cpus_used: 2,
                memory_used_mb: 2048,
                vms: 2,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refreshed.cpus_used, 2);
    assert_eq!(refreshed.vms, 2);
    assert_eq!(refreshed.state, NodeState::Online);
}

#[tokio::test]
async fn test_drain_excludes_from_scheduling() {
    let mut module = new_module();
    let node = module.register(sample_node("d", 8, 16384, vec!["gpu".to_string()], 30)).await.unwrap();
    module.register(sample_node("e", 8, 16384, Vec::new(), 30)).await.unwrap();

    let fit = module.best_fit(2, 4096, &[], &[]).await.unwrap();
    assert_eq!(fit, node.id);

    module.drain(&node.id, true).await.unwrap();
    let fit2 = module.best_fit(2, 4096, &[], &[]).await.unwrap();
    assert_ne!(fit2, node.id);
}

#[tokio::test]
async fn test_best_fit_picks_least_loaded() {
    let mut module = new_module();
    let busy = module.register(sample_node("f", 8, 16384, Vec::new(), 30)).await.unwrap();
    let idle = module.register(sample_node("g", 8, 16384, Vec::new(), 30)).await.unwrap();

    // Half-load f (4 of 8 cpus) so g is the lighter pick.
    module.reserve(&busy.id, 4, 0).await.unwrap();

    let fit = module.best_fit(2, 4096, &[], &[]).await.unwrap();
    assert_eq!(fit, idle.id);
}

#[tokio::test]
async fn test_affinity_and_anti_affinity() {
    let mut module = new_module();
    module.register(sample_node("h", 8, 16384, vec!["ssd".to_string()], 30)).await.unwrap();
    let plain = module.register(sample_node("i", 8, 16384, Vec::new(), 30)).await.unwrap();

    let fit = module.best_fit(2, 4096, &["ssd".to_string()], &[]).await.unwrap();
    assert_ne!(fit, plain.id);

    let fit2 = module.best_fit(2, 4096, &[], &["ssd".to_string()]).await.unwrap();
    assert_eq!(fit2, plain.id);
}

#[tokio::test]
async fn test_reserve_and_release_capacity() {
    let mut module = new_module();
    let node = module.register(sample_node("j", 4, 4096, Vec::new(), 30)).await.unwrap();
    let id = node.id.clone();

    module.reserve(&id, 2, 2048).await.unwrap();
    let n = module.get(&id).await.unwrap().unwrap();
    assert_eq!(n.cpus_used, 2);
    assert_eq!(n.memory_used_mb, 2048);
    assert_eq!(n.vms, 1);

    assert!(module.reserve(&id, 4, 4096).await.is_err());

    module.release(&id, 2, 2048).await.unwrap();
    let n = module.get(&id).await.unwrap().unwrap();
    assert_eq!(n.cpus_used, 0);
    assert_eq!(n.vms, 0);
}

#[tokio::test]
async fn test_no_suitable_node_errors() {
    let module = new_module();
    let err = module.best_fit(2, 4096, &[], &[]).await.unwrap_err();
    assert!(err.to_string().contains("no suitable node"));
}