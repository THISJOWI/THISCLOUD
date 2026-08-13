use crate::auth::rbac::{CreateSet, ReadSet, RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::core::AppError;
use crate::s3::module::S3Module;
use crate::s3::S3AccessKey;
use crate::s3::S3Bucket;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct S3ApiState {
    pub module: Arc<Mutex<S3Module>>,
}

impl S3ApiState {
    pub fn new(module: Arc<Mutex<S3Module>>) -> Self {
        Self { module }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateBucketRequest {
    pub name: String,
}

pub fn app(state: S3ApiState) -> Router {
    let read = Router::new()
        .route("/s3/buckets", get(list_buckets))
        .route("/s3/buckets/:name", get(get_bucket))
        .route("/s3/credentials", get(list_credentials))
        .route_layer(middleware::from_extractor::<RequireRole<ReadSet>>());

    let create = Router::new()
        .route("/s3/buckets", post(create_bucket))
        .route("/s3/credentials", post(issue_credentials))
        .route_layer(middleware::from_extractor::<RequireRole<CreateSet>>());

    let mutate = Router::new()
        .route("/s3/buckets/:name", delete(delete_bucket))
        .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>());

    read.merge(create).merge(mutate).with_state(state)
}

async fn list_buckets(
    State(state): State<S3ApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<S3Bucket>>, AppError> {
    let module = state.module.lock().await;
    let buckets = module.list_buckets(&ctx.tenant_id).await?;
    Ok(Json(buckets))
}

async fn create_bucket(
    State(state): State<S3ApiState>,
    ctx: TenantContext,
    Json(req): Json<CreateBucketRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.name.is_empty() {
        return Err(AppError::Validation("bucket name is required".into()));
    }
    let mut module = state.module.lock().await;
    let bucket = module.create_bucket(&ctx.tenant_id, &req.name).await?;
    Ok((StatusCode::CREATED, Json(bucket)))
}

async fn get_bucket(
    State(state): State<S3ApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<Json<S3Bucket>, AppError> {
    let module = state.module.lock().await;
    let bucket = module.get_bucket(&ctx.tenant_id, &name).await?;
    Ok(Json(bucket))
}

async fn delete_bucket(
    State(state): State<S3ApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete_bucket(&ctx.tenant_id, &name).await?;
    Ok(StatusCode::OK)
}

async fn issue_credentials(
    State(state): State<S3ApiState>,
    ctx: TenantContext,
) -> Result<impl IntoResponse, AppError> {
    let mut module = state.module.lock().await;
    let key = module.issue_credentials(&ctx.tenant_id).await?;
    Ok((StatusCode::CREATED, Json(key)))
}

async fn list_credentials(
    State(state): State<S3ApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<S3AccessKey>>, AppError> {
    let module = state.module.lock().await;
    let keys = module.list_credentials(&ctx.tenant_id).await?;
    Ok(Json(keys))
}