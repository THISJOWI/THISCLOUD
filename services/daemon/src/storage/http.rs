use crate::auth::rbac::{CreateSet, ReadSet, RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::storage::module::StorageModule;
use crate::storage::StoragePool;
use crate::core::AppError;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct StorageApiState {
    pub module: Arc<Mutex<StorageModule>>,
}

impl StorageApiState {
    pub fn new(module: Arc<Mutex<StorageModule>>) -> Self {
        Self { module }
    }
}

pub fn app(state: StorageApiState) -> Router {
    let read = Router::new()
        .route("/storage/pools", get(list_pools))
        .route("/storage/pools/:name", get(get_pool))
        .route_layer(middleware::from_extractor::<RequireRole<ReadSet>>());

    let create = Router::new()
        .route("/storage/pools", post(create_pool))
        .route_layer(middleware::from_extractor::<RequireRole<CreateSet>>());

    let mutate = Router::new()
        .route("/storage/pools/:name", delete(delete_pool))
        .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>());

    read.merge(create).merge(mutate).with_state(state)
}

async fn list_pools(
    State(state): State<StorageApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<StoragePool>>, AppError> {
    let module = state.module.lock().await;
    let pools = module.list_pools(&ctx.tenant_id).await?;
    Ok(Json(pools))
}

async fn create_pool(
    State(state): State<StorageApiState>,
    ctx: TenantContext,
    Json(pool): Json<StoragePool>,
) -> Result<impl IntoResponse, AppError> {
    if pool.replication == 0 {
        return Err(AppError::Validation(
            "replication must be greater than 0".into(),
        ));
    }

    let mut module = state.module.lock().await;
    module.create_pool(&ctx.tenant_id, pool.clone()).await?;
    Ok((StatusCode::CREATED, Json(pool)))
}

async fn get_pool(
    State(state): State<StorageApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<Json<StoragePool>, AppError> {
    let module = state.module.lock().await;
    let pool = module.get_pool(&ctx.tenant_id, &name).await?;
    Ok(Json(pool))
}

async fn delete_pool(
    State(state): State<StorageApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete_pool(&ctx.tenant_id, &name).await?;
    Ok(StatusCode::OK)
}
