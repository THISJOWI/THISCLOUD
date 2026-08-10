use crate::network::module::NetworkModule;
use crate::network::LogicalNetwork;
use crate::core::AppError;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct NetworkApiState {
    pub module: Arc<Mutex<NetworkModule>>,
}

impl NetworkApiState {
    pub fn new(module: Arc<Mutex<NetworkModule>>) -> Self {
        Self { module }
    }
}

pub fn app(state: NetworkApiState) -> Router {
    Router::new()
        .route("/networks", get(list_networks).post(create_network))
        .route("/networks/:id", get(get_network).delete(delete_network))
        .with_state(state)
}

/// Validate that a string looks like a CIDR notation (e.g. "10.0.0.0/24").
fn validate_cidr(cidr: &str) -> Result<(), AppError> {
    let (ip_str, prefix_str) = cidr
        .split_once('/')
        .ok_or_else(|| AppError::Validation("cidr must be in format IP/prefix (e.g. 10.0.0.0/24)".into()))?;

    // Validate the IP part
    let ip: std::net::IpAddr = ip_str
        .parse()
        .map_err(|_| AppError::Validation(format!("invalid IP address in cidr: {ip_str}")))?;

    // Validate the prefix length
    let prefix: u8 = prefix_str
        .parse()
        .map_err(|_| AppError::Validation(format!("invalid prefix length in cidr: {prefix_str}")))?;

    match ip {
        std::net::IpAddr::V4(_) => {
            if prefix > 32 {
                return Err(AppError::Validation(
                    "IPv4 prefix must be between 0 and 32".into(),
                ));
            }
        }
        std::net::IpAddr::V6(_) => {
            if prefix > 128 {
                return Err(AppError::Validation(
                    "IPv6 prefix must be between 0 and 128".into(),
                ));
            }
        }
    }

    Ok(())
}

async fn list_networks(
    State(state): State<NetworkApiState>,
) -> Result<Json<Vec<LogicalNetwork>>, AppError> {
    let module = state.module.lock().await;
    let networks = module.list_networks().await?;
    Ok(Json(networks))
}

async fn create_network(
    State(state): State<NetworkApiState>,
    Json(mut net): Json<LogicalNetwork>,
) -> Result<impl IntoResponse, AppError> {
    if net.id.is_empty() {
        net.id = Uuid::new_v4().to_string();
    }
    validate_cidr(&net.cidr)?;

    let mut module = state.module.lock().await;
    module.create_network(&mut net).await?;
    Ok((StatusCode::CREATED, Json(net)))
}

async fn get_network(
    State(state): State<NetworkApiState>,
    Path(id): Path<String>,
) -> Result<Json<LogicalNetwork>, AppError> {
    let module = state.module.lock().await;
    let net = module.get_network(&id).await?;
    Ok(Json(net))
}

async fn delete_network(
    State(state): State<NetworkApiState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete_network(&id).await?;
    Ok(StatusCode::OK)
}
