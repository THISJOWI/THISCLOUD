use tower::ServiceExt;

/// A fresh daemon with no identity seeds its own node entry so cluster state
/// survives restarts and the scheduler always has a candidate.
#[tokio::test]
async fn test_agent_seeds_local_master() {
    let config = thiscloudd::config::ThisCloudConfig::default();
    let mut daemon = thiscloudd::core::Daemon::new(config, None);
    daemon.start().await.unwrap();

    let app = daemon.http_router();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/nodes")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let nodes: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(nodes.len(), 1, "expected exactly one seeded node");

    let n = &nodes[0];
    assert_eq!(n["id"], "master-1");
    assert_eq!(n["role"], "master");
    assert_eq!(n["state"], "online");
    assert!(
        n["cpus_total"].as_u64().unwrap() >= 1,
        "agent should report real capacity"
    );
    assert!(n["last_seen_secs"].as_u64().unwrap() > 0);
}

/// A node explicitly configured as a worker (with a master list) must NOT seed
/// itself into the local store — registration happens through the master.
#[tokio::test]
async fn test_agent_worker_does_not_seed_locally() {
    let config_content = r#"
[node]
id = "worker-7"
role = "worker"
master = "http://192.168.1.12:8080"
"#;
    let config: thiscloudd::config::ThisCloudConfig = toml::from_str(config_content).unwrap();
    let mut daemon = thiscloudd::core::Daemon::new(config, None);
    daemon.start().await.unwrap();

    let app = daemon.http_router();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/nodes")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let nodes: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(
        nodes.is_empty(),
        "worker with a master list must not self-seed locally"
    );
}

/// The list and single-node lookups must agree on effective state. A node whose
/// TTL has elapsed reports offline through both endpoints; a fresh one is
/// online through both.
#[tokio::test]
async fn test_node_state_consistent_between_list_and_get() {
    let config = thiscloudd::config::ThisCloudConfig::default();
    let mut daemon = thiscloudd::core::Daemon::new(config, None);
    daemon.start().await.unwrap();

    let app = daemon.http_router();

    let list_resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/nodes")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(list_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let nodes: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(nodes.len(), 1, "expected exactly one seeded node");
    let listed = &nodes[0];

    let get_resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/nodes/{}", listed["id"].as_str().unwrap()))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), axum::http::StatusCode::OK);
    let body = axum::body::to_bytes(get_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let got: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        got["state"], listed["state"],
        "GET /nodes/:id must report the same effective state as the list"
    );
    assert_eq!(got["id"], listed["id"]);
}