use crate::auth::rbac::{CreateSet, ReadSet, RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::compute::vm::{ConsoleInfo, DiskConfig, HotplugRequest, VmConfig};
use crate::compute::ComputeModule;
use crate::core::AppError;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use futures_util::StreamExt;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiState {
    pub module: Arc<Mutex<ComputeModule>>,
}

impl ApiState {
    pub fn new(module: Arc<Mutex<ComputeModule>>) -> Self {
        Self { module }
    }
}

#[derive(Deserialize)]
pub struct SnapshotReq {
    pub name: String,
}

#[derive(Deserialize)]
pub struct RestoreReq {
    pub snapshot_id: String,
}

#[derive(Deserialize)]
pub struct CloneReq {
    pub name: String,
}

#[derive(Deserialize)]
pub struct ResizeReq {
    #[serde(default)]
    pub cpus: u32,
    #[serde(default)]
    pub memory_mb: u32,
}

#[derive(Deserialize)]
pub struct DiskReq {
    #[serde(default)]
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub size_gb: u32,
}

#[derive(Deserialize)]
pub struct NicReq {
    pub tap: String,
}

#[derive(Deserialize)]
pub struct MigrateReq {
    pub target_node: String,
}

#[derive(Deserialize)]
pub struct MemoryReq {
    pub target_mb: u32,
}

pub fn app(state: ApiState) -> Router {
    let read = Router::new()
        .route("/vms", get(list_vms))
        .route("/vms/:id", get(get_vm))
        .route_layer(middleware::from_extractor::<RequireRole<ReadSet>>());

    let create = Router::new()
        .route("/vms", post(create_vm))
        .route_layer(middleware::from_extractor::<RequireRole<CreateSet>>());

    let mutate = Router::new()
        .route("/vms/:id", delete(delete_vm))
        .route("/vms/:id/start", post(start_vm))
        .route("/vms/:id/stop", post(stop_vm))
        .route("/vms/:id/snapshot", post(snapshot_vm))
        .route("/vms/:id/restore", post(restore_vm))
        .route("/vms/:id/clone", post(clone_vm))
        .route("/vms/:id/resize", post(resize_vm))
        .route("/vms/:id/memory", post(resize_memory_vm))
        .route("/vms/:id/migrate", post(migrate_vm))
        .route("/vms/:id/hotplug", post(hotplug_vm))
        .route("/vms/:id/disks", put(attach_disk))
        .route("/vms/:id/disks/:disk_id", delete(detach_disk))
        .route("/vms/:id/nics", put(attach_nic))
        .route("/vms/:id/nics/:tap", delete(detach_nic))
        .route("/vms/:id/console", get(console_vm))
        .route("/vms/:id/console/ws", get(console_ws))
        .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>());

    read.merge(create).merge(mutate).with_state(state)
}

async fn list_vms(
    State(state): State<ApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<VmConfig>>, AppError> {
    let module = state.module.lock().await;
    let vms = module.list_vms(&ctx.tenant_id).await?;
    Ok(Json(vms))
}

async fn create_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Json(mut vm): Json<VmConfig>,
) -> Result<impl IntoResponse, AppError> {
    if vm.id.is_empty() {
        vm.id = Uuid::new_v4().to_string();
    }
    if vm.cpus == 0 {
        return Err(AppError::Validation("cpus must be greater than 0".into()));
    }
    if vm.memory_mb == 0 {
        return Err(AppError::Validation(
            "memory_mb must be greater than 0".into(),
        ));
    }
    if vm.disk_path.is_empty() {
        vm.disk_path = format!("/var/lib/thiscloud/vms/{}.qcow2", vm.name);
    }

    let mut module = state.module.lock().await;
    module.create_vm(&ctx.tenant_id, vm.clone()).await?;
    Ok((StatusCode::CREATED, Json(vm)))
}

async fn get_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<Json<VmConfig>, AppError> {
    let module = state.module.lock().await;
    let vm = module.get_vm(&ctx.tenant_id, &id).await?;
    Ok(Json(vm))
}

async fn delete_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete_vm(&ctx.tenant_id, &id).await?;
    Ok(StatusCode::OK)
}

async fn start_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<Json<VmConfig>, AppError> {
    let mut module = state.module.lock().await;
    module.start_vm(&ctx.tenant_id, &id).await?;
    let vm = module.get_vm(&ctx.tenant_id, &id).await?;
    Ok(Json(vm))
}

async fn stop_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<Json<VmConfig>, AppError> {
    let mut module = state.module.lock().await;
    module.stop_vm(&ctx.tenant_id, &id).await?;
    let vm = module.get_vm(&ctx.tenant_id, &id).await?;
    Ok(Json(vm))
}

// ── T1.1: full lifecycle handlers ─────────────────────────

async fn snapshot_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<SnapshotReq>,
) -> Result<impl IntoResponse, AppError> {
    let mut module = state.module.lock().await;
    let snap = module.snapshot_vm(&ctx.tenant_id, &id, &req.name).await?;
    Ok((StatusCode::CREATED, Json(snap)))
}

async fn restore_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<RestoreReq>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.restore_snapshot(&ctx.tenant_id, &id, &req.snapshot_id).await?;
    Ok(StatusCode::OK)
}

async fn clone_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<CloneReq>,
) -> Result<impl IntoResponse, AppError> {
    let mut module = state.module.lock().await;
    let vm = module.clone_vm(&ctx.tenant_id, &id, &req.name).await?;
    Ok((StatusCode::CREATED, Json(vm)))
}

async fn resize_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<ResizeReq>,
) -> Result<Json<VmConfig>, AppError> {
    if req.cpus == 0 && req.memory_mb == 0 {
        return Err(AppError::Validation(
            "cpus or memory_mb must be provided".into(),
        ));
    }
    let mut module = state.module.lock().await;
    let vm = module
        .resize_vm(&ctx.tenant_id, &id, req.cpus, req.memory_mb)
        .await?;
    Ok(Json(vm))
}

async fn migrate_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<MigrateReq>,
) -> Result<Json<VmConfig>, AppError> {
    let mut module = state.module.lock().await;
    let vm = module
        .migrate_vm(&ctx.tenant_id, &id, &req.target_node)
        .await?;
    Ok(Json(vm))
}

async fn resize_memory_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<MemoryReq>,
) -> Result<Json<VmConfig>, AppError> {
    let mut module = state.module.lock().await;
    let vm = module
        .resize_memory(&ctx.tenant_id, &id, req.target_mb)
        .await?;
    Ok(Json(vm))
}

async fn hotplug_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<HotplugRequest>,
) -> Result<Json<VmConfig>, AppError> {
    let mut module = state.module.lock().await;
    let vm = module.hotplug_vm(&ctx.tenant_id, &id, &req).await?;
    Ok(Json(vm))
}

async fn attach_disk(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<DiskReq>,
) -> Result<impl IntoResponse, AppError> {
    let mut module = state.module.lock().await;
    let disk = module
        .attach_disk(
            &ctx.tenant_id,
            &id,
            DiskConfig {
                id: req.id,
                path: req.path,
                size_gb: req.size_gb,
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(disk)))
}

async fn detach_disk(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path((id, disk_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.detach_disk(&ctx.tenant_id, &id, &disk_id).await?;
    Ok(StatusCode::OK)
}

async fn attach_nic(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<NicReq>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.attach_nic(&ctx.tenant_id, &id, &req.tap).await?;
    Ok(StatusCode::OK)
}

async fn detach_nic(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path((id, tap)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.detach_nic(&ctx.tenant_id, &id, &tap).await?;
    Ok(StatusCode::OK)
}

async fn console_vm(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<Json<ConsoleInfo>, AppError> {
    let module = state.module.lock().await;
    let info = module.console_url(&ctx.tenant_id, &id).await?;
    Ok(Json(info))
}

async fn console_ws(
    State(state): State<ApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<axum::response::Response, AppError> {
    {
        let module = state.module.lock().await;
        module.get_vm(&ctx.tenant_id, &id).await?;
    }
    Ok(ws.on_upgrade(move |socket| console_session(socket, id)))
}

async fn console_session(mut socket: WebSocket, id: String) {
    let banner = format!(
        "\r\nTHISCLOUD console — vm {}\r\nType commands (mock backend echoes input).\r\n\r\n",
        id
    );
    if socket.send(Message::Text(banner)).await.is_err() {
        return;
    }

    while let Some(msg) = socket.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if socket.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
}
