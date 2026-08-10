use crate::storage::module::StorageModule;
use crate::storage::StoragePool;
use crate::core::AppError;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
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
    Router::new()
        .route("/storage/pools", get(list_pools).post(create_pool))
        .route("/storage/pools/:name", get(get_pool).delete(delete_pool))
        .with_state(state)
}

async fn list_pools(
    State(state): State<StorageApiState>,
) -> Result<Json<Vec<StoragePool>>, AppError> {
    let module = state.module.lock().await;
    let pools = module.list_pools().await?;
    Ok(Json(pools))
}

async fn create_pool(
    State(state): State<StorageApiState>,
    Json(pool): Json<StoragePool>,
) -> Result<impl IntoResponse, AppError> {
    if pool.replication == 0 {
        return Err(AppError::Validation(
            "replication must be greater than 0".into(),
        ));
    }

    let mut module = state.module.lock().await;
    module.create_pool(pool.clone()).await?;
    Ok((StatusCode::CREATED, Json(pool)))
}

async fn get_pool(
    State(state): State<StorageApiState>,
    Path(name): Path<String>,
) -> Result<Json<StoragePool>, AppError> {
    let module = state.module.lock().await;
    let pool = module.get_pool(&name).await?;
    Ok(Json(pool))
}

async fn delete_pool(
    State(state): State<StorageApiState>,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete_pool(&name).await?;
    Ok(StatusCode::OK)
}
