use crate::auth::rbac::{CreateSet, ReadSet, RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::core::AppError;
use crate::image::Image;
use crate::image::module::ImageModule;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ImageApiState {
    pub module: Arc<Mutex<ImageModule>>,
}

impl ImageApiState {
    pub fn new(module: Arc<Mutex<ImageModule>>) -> Self {
        Self { module }
    }
}

pub fn app(state: ImageApiState) -> Router {
    let read = Router::new()
        .route("/images", get(list_images))
        .route("/images/:id", get(get_image))
        .route_layer(middleware::from_extractor::<RequireRole<ReadSet>>());

    let create = Router::new()
        .route("/images", post(register_image))
        .route_layer(middleware::from_extractor::<RequireRole<CreateSet>>());

    let mutate = Router::new()
        .route("/images/:id", delete(remove_image))
        .route("/images/:id/upload", put(upload_image))
        .route("/images/:id/template", put(set_template))
        .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>());

    read.merge(create).merge(mutate).with_state(state)
}

async fn list_images(
    State(state): State<ImageApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<Image>>, AppError> {
    let module = state.module.lock().await;
    let images = module.list(&ctx.tenant_id).await?;
    Ok(Json(images))
}

async fn register_image(
    State(state): State<ImageApiState>,
    ctx: TenantContext,
    Json(mut image): Json<Image>,
) -> Result<impl IntoResponse, AppError> {
    let mut module = state.module.lock().await;
    let registered = module.register(&ctx.tenant_id, &mut image).await?;
    Ok((StatusCode::CREATED, Json(registered)))
}

async fn get_image(
    State(state): State<ImageApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<Json<Image>, AppError> {
    let module = state.module.lock().await;
    let image = match module.get(&ctx.tenant_id, &id).await {
        Ok(img) => img,
        Err(_) => module
            .get_by_name(&ctx.tenant_id, &id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("image {id} not found")))?,
    };
    Ok(Json(image))
}

/// Accepts raw artifact bytes (e.g. an uploaded ISO/qcow2) for an
/// already-registered image. The client first POSTs /images with metadata,
/// then PUTs the file here. Content-Type is application/octet-stream.
async fn upload_image(
    State(state): State<ImageApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<Image>, AppError> {
    let mut module = state.module.lock().await;
    let image = module.upload(&ctx.tenant_id, &id, &body).await?;
    Ok(Json(image))
}

async fn remove_image(
    State(state): State<ImageApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.remove(&ctx.tenant_id, &id).await?;
    Ok(StatusCode::OK)
}

async fn set_template(
    State(state): State<ImageApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Image>, AppError> {
    let template = req
        .get("template")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut module = state.module.lock().await;
    let image = module.set_template(&ctx.tenant_id, &id, template).await?;
    Ok(Json(image))
}