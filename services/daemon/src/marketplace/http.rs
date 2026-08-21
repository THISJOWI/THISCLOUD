use crate::auth::rbac::{CreateSet, ReadSet, RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::marketplace::module::MarketplaceModule;
use crate::marketplace::MarketplaceApp;
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
pub struct MarketplaceApiState {
    pub module: Arc<Mutex<MarketplaceModule>>,
}

impl MarketplaceApiState {
    pub fn new(module: Arc<Mutex<MarketplaceModule>>) -> Self {
        Self { module }
    }
}

pub fn app(state: MarketplaceApiState) -> Router {
    let read = Router::new()
        .route("/marketplace/apps", get(list_apps))
        .route("/marketplace/apps/:id", get(get_app))
        .route_layer(middleware::from_extractor::<RequireRole<ReadSet>>());

    let create = Router::new()
        .route("/marketplace/apps", post(install_app))
        .route_layer(middleware::from_extractor::<RequireRole<CreateSet>>());

    let mutate = Router::new()
        .route("/marketplace/apps/:id", delete(uninstall_app))
        .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>());

    read.merge(create).merge(mutate).with_state(state)
}

async fn list_apps(
    State(state): State<MarketplaceApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<MarketplaceApp>>, AppError> {
    let module = state.module.lock().await;
    let apps = module.list(&ctx.tenant_id).await?;
    Ok(Json(apps))
}

async fn install_app(
    State(state): State<MarketplaceApiState>,
    ctx: TenantContext,
    Json(mut app): Json<MarketplaceApp>,
) -> Result<impl IntoResponse, AppError> {
    let mut module = state.module.lock().await;
    let installed = module.install(&ctx.tenant_id, &mut app).await?;
    Ok((StatusCode::CREATED, Json(installed)))
}

async fn get_app(
    State(state): State<MarketplaceApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<Json<MarketplaceApp>, AppError> {
    let module = state.module.lock().await;
    let app = module.get(&ctx.tenant_id, &id).await?;
    Ok(Json(app))
}

async fn uninstall_app(
    State(state): State<MarketplaceApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.uninstall(&ctx.tenant_id, &id).await?;
    Ok(StatusCode::OK)
}
