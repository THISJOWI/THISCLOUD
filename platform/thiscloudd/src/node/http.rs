use crate::auth::rbac::{CreateSet, ReadSet, RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::core::AppError;
use crate::node::model::{Node, NodeHeartbeat};
use crate::node::NodeModule;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct NodeApiState {
    pub module: Arc<Mutex<NodeModule>>,
}

impl NodeApiState {
    pub fn new(module: Arc<Mutex<NodeModule>>) -> Self {
        Self { module }
    }
}

#[derive(Deserialize)]
pub struct DrainReq {
    pub drain: bool,
}

pub fn app(state: NodeApiState) -> Router {
    let read = Router::new()
        .route("/nodes", get(list_nodes))
        .route("/nodes/:id", get(get_node))
        .route_layer(middleware::from_extractor::<RequireRole<ReadSet>>());

    let create = Router::new()
        .route("/nodes", post(register_node))
        .route_layer(middleware::from_extractor::<RequireRole<CreateSet>>());

    let mutate = Router::new()
        .route("/nodes/:id", delete(delete_node))
        .route("/nodes/:id/drain", put(drain_node))
        .route("/nodes/:id/heartbeat", post(heartbeat_node))
        .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>());

    read.merge(create).merge(mutate).with_state(state)
}

async fn list_nodes(
    State(state): State<NodeApiState>,
) -> Result<Json<Vec<Node>>, AppError> {
    let module = state.module.lock().await;
    let nodes = module.list().await?;
    Ok(Json(nodes))
}

async fn get_node(
    State(state): State<NodeApiState>,
    Path(id): Path<String>,
) -> Result<Json<Node>, AppError> {
    let module = state.module.lock().await;
    let mut node = module
        .get(&id)
        .await?
        .ok_or_else(|| AppError::NotFound("node not found".into()))?;
    // Apply the same TTL-based state derivation the list endpoint uses, so a
    // single-node lookup reports the same effective state as the list view.
    node.state = NodeModule::effective_state(&node);
    Ok(Json(node))
}

async fn register_node(
    State(state): State<NodeApiState>,
    _ctx: TenantContext,
    Json(node): Json<Node>,
) -> Result<impl IntoResponse, AppError> {
    let mut module = state.module.lock().await;
    let node = module.register(node).await?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn delete_node(
    State(state): State<NodeApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete(&id).await?;
    Ok(StatusCode::OK)
}

async fn drain_node(
    State(state): State<NodeApiState>,
    Path(id): Path<String>,
    Json(req): Json<DrainReq>,
) -> Result<Json<Node>, AppError> {
    let mut module = state.module.lock().await;
    let node = module
        .drain(&id, req.drain)
        .await?
        .ok_or_else(|| AppError::NotFound("node not found".into()))?;
    Ok(Json(node))
}

async fn heartbeat_node(
    State(state): State<NodeApiState>,
    Path(id): Path<String>,
    Json(hb): Json<NodeHeartbeat>,
) -> Result<Json<Node>, AppError> {
    let mut module = state.module.lock().await;
    let node = module
        .heartbeat(&id, hb)
        .await?
        .ok_or_else(|| AppError::NotFound("node not found".into()))?;
    Ok(Json(node))
}