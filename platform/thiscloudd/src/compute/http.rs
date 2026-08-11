use crate::auth::rbac::{CreateSet, ReadSet, RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::compute::vm::VmConfig;
use crate::compute::ComputeModule;
use crate::core::AppError;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
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
