use crate::marketplace::module::MarketplaceModule;
use crate::marketplace::MarketplaceApp;
use crate::core::AppError;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
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
    Router::new()
        .route("/marketplace/apps", get(list_apps).post(install_app))
        .route("/marketplace/apps/:id", get(get_app).delete(uninstall_app))
        .with_state(state)
}

async fn list_apps(
    State(state): State<MarketplaceApiState>,
) -> Result<Json<Vec<MarketplaceApp>>, AppError> {
    let module = state.module.lock().await;
    let apps = module.list().await?;
    Ok(Json(apps))
}

async fn install_app(
    State(state): State<MarketplaceApiState>,
    Json(mut app): Json<MarketplaceApp>,
) -> Result<impl IntoResponse, AppError> {
    let mut module = state.module.lock().await;
    let installed = module.install(&mut app).await?;
    Ok((StatusCode::CREATED, Json(installed)))
}

async fn get_app(
    State(state): State<MarketplaceApiState>,
    Path(id): Path<String>,
) -> Result<Json<MarketplaceApp>, AppError> {
    let module = state.module.lock().await;
    let app = module.get(&id).await?;
    Ok(Json(app))
}

async fn uninstall_app(
    State(state): State<MarketplaceApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.uninstall(&id).await?;
    Ok(StatusCode::OK)
}
