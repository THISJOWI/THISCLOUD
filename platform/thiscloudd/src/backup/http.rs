use crate::auth::rbac::{CreateSet, ReadSet, RequireRole};
use crate::backup::BackupService;
use crate::core::AppError;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::sync::Arc;

#[derive(Clone)]
pub struct BackupApiState {
    pub service: Arc<BackupService>,
}

impl BackupApiState {
    pub fn new(service: Arc<BackupService>) -> Self {
        Self { service }
    }
}

pub fn app(state: BackupApiState) -> Router {
    let read = Router::new()
        .route("/backup", get(list_snapshots))
        .route_layer(middleware::from_extractor::<RequireRole<ReadSet>>());

    let create = Router::new()
        .route("/backup", post(create_snapshot))
        .route("/backup/:name/restore", post(restore_snapshot))
        .route_layer(middleware::from_extractor::<RequireRole<CreateSet>>());

    read.merge(create).with_state(state)
}

async fn create_snapshot(
    State(state): State<BackupApiState>,
) -> Result<impl IntoResponse, AppError> {
    let info = state.service.create_snapshot().await.map_err(AppError::from)?;
    Ok((StatusCode::CREATED, Json(info)))
}

async fn list_snapshots(
    State(state): State<BackupApiState>,
) -> Result<Json<Vec<crate::backup::SnapshotInfo>>, AppError> {
    let snapshots = state.service.list_snapshots().map_err(AppError::from)?;
    Ok(Json(snapshots))
}

async fn restore_snapshot(
    State(state): State<BackupApiState>,
    Path(name): Path<String>,
) -> Result<Json<crate::backup::SnapshotInfo>, AppError> {
    let info = state
        .service
        .restore_snapshot(&name)
        .await
        .map_err(AppError::from)?;
    Ok(Json(info))
}