use crate::auth::rbac::{CreateSet, ReadSet, RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::network::module::NetworkModule;
use crate::network::{DhcpServer, FloatingIp, LogicalNetwork, VirtualRouter};
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
pub struct NetworkApiState {
    pub module: Arc<Mutex<NetworkModule>>,
}

impl NetworkApiState {
    pub fn new(module: Arc<Mutex<NetworkModule>>) -> Self {
        Self { module }
    }
}

pub fn app(state: NetworkApiState) -> Router {
    let read = Router::new()
        .route("/networks", get(list_networks))
        .route("/networks/:id", get(get_network))
        .route("/network/routers", get(list_routers))
        .route("/network/routers/:name", get(get_router))
        .route("/network/dhcp", get(list_dhcp))
        .route("/network/dhcp/:name", get(get_dhcp))
        .route("/network/floating-ips", get(list_floating_ips))
        .route("/network/floating-ips/:name", get(get_floating_ip))
        .route_layer(middleware::from_extractor::<RequireRole<ReadSet>>());

    let create = Router::new()
        .route("/networks", post(create_network))
        .route("/network/routers", post(create_router))
        .route("/network/dhcp", post(create_dhcp))
        .route("/network/floating-ips", post(allocate_floating_ip))
        .route_layer(middleware::from_extractor::<RequireRole<CreateSet>>());

    let mutate = Router::new()
        .route("/networks/:id", delete(delete_network))
        .route("/network/routers/:name", delete(delete_router))
        .route("/network/dhcp/:name", delete(delete_dhcp))
        .route("/network/floating-ips/:name", delete(deallocate_floating_ip))
        .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>());

    read.merge(create).merge(mutate).with_state(state)
}

fn validate_cidr(cidr: &str) -> Result<(), AppError> {
    let (ip_str, prefix_str) = cidr
        .split_once('/')
        .ok_or_else(|| AppError::Validation("cidr must be in format IP/prefix (e.g. 10.0.0.0/24)".into()))?;

    let ip: std::net::IpAddr = ip_str
        .parse()
        .map_err(|_| AppError::Validation(format!("invalid IP address in cidr: {ip_str}")))?;

    let prefix: u8 = prefix_str
        .parse()
        .map_err(|_| AppError::Validation(format!("invalid prefix length in cidr: {prefix_str}")))?;

    match ip {
        std::net::IpAddr::V4(_) => {
            if prefix > 32 {
                return Err(AppError::Validation("IPv4 prefix must be between 0 and 32".into()));
            }
        }
        std::net::IpAddr::V6(_) => {
            if prefix > 128 {
                return Err(AppError::Validation("IPv6 prefix must be between 0 and 128".into()));
            }
        }
    }

    Ok(())
}

async fn list_networks(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<LogicalNetwork>>, AppError> {
    let module = state.module.lock().await;
    let networks = module.list_networks(&ctx.tenant_id).await?;
    Ok(Json(networks))
}

async fn create_network(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Json(mut net): Json<LogicalNetwork>,
) -> Result<impl IntoResponse, AppError> {
    if net.id.is_empty() {
        net.id = Uuid::new_v4().to_string();
    }
    validate_cidr(&net.cidr)?;

    let mut module = state.module.lock().await;
    module.create_network(&ctx.tenant_id, &mut net).await?;
    Ok((StatusCode::CREATED, Json(net)))
}

async fn get_network(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<Json<LogicalNetwork>, AppError> {
    let module = state.module.lock().await;
    let net = module.get_network(&ctx.tenant_id, &id).await?;
    Ok(Json(net))
}

async fn delete_network(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete_network(&ctx.tenant_id, &id).await?;
    Ok(StatusCode::OK)
}

// --- Virtual routers ---

async fn list_routers(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<VirtualRouter>>, AppError> {
    let module = state.module.lock().await;
    let routers = module.list_routers(&ctx.tenant_id).await?;
    Ok(Json(routers))
}

async fn create_router(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Json(mut router): Json<VirtualRouter>,
) -> Result<impl IntoResponse, AppError> {
    if router.id.is_empty() {
        router.id = Uuid::new_v4().to_string();
    }
    let mut module = state.module.lock().await;
    module.create_router(&ctx.tenant_id, &mut router).await?;
    Ok((StatusCode::CREATED, Json(router)))
}

async fn get_router(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<Json<VirtualRouter>, AppError> {
    let module = state.module.lock().await;
    let router = match module.get_router(&ctx.tenant_id, &name).await {
        Ok(r) => r,
        Err(_) => module
            .list_routers(&ctx.tenant_id)
            .await?
            .into_iter()
            .find(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(format!("router {name} not found")))?,
    };
    Ok(Json(router))
}

async fn delete_router(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete_router(&ctx.tenant_id, &name).await?;
    Ok(StatusCode::OK)
}

// --- DHCP servers ---

async fn list_dhcp(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<DhcpServer>>, AppError> {
    let module = state.module.lock().await;
    let servers = module.list_dhcp(&ctx.tenant_id).await?;
    Ok(Json(servers))
}

async fn create_dhcp(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Json(mut dhcp): Json<DhcpServer>,
) -> Result<impl IntoResponse, AppError> {
    if dhcp.id.is_empty() {
        dhcp.id = Uuid::new_v4().to_string();
    }
    let mut module = state.module.lock().await;
    module.create_dhcp(&ctx.tenant_id, &mut dhcp).await?;
    Ok((StatusCode::CREATED, Json(dhcp)))
}

async fn get_dhcp(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<Json<DhcpServer>, AppError> {
    let module = state.module.lock().await;
    let dhcp = match module.get_dhcp(&ctx.tenant_id, &name).await {
        Ok(d) => d,
        Err(_) => module
            .list_dhcp(&ctx.tenant_id)
            .await?
            .into_iter()
            .find(|d| d.name == name)
            .ok_or_else(|| AppError::NotFound(format!("dhcp server {name} not found")))?,
    };
    Ok(Json(dhcp))
}

async fn delete_dhcp(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module.delete_dhcp(&ctx.tenant_id, &name).await?;
    Ok(StatusCode::OK)
}

// --- Floating IPs ---

async fn list_floating_ips(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
) -> Result<Json<Vec<FloatingIp>>, AppError> {
    let module = state.module.lock().await;
    let fips = module.list_floating_ips(&ctx.tenant_id).await?;
    Ok(Json(fips))
}

async fn allocate_floating_ip(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Json(mut fip): Json<FloatingIp>,
) -> Result<impl IntoResponse, AppError> {
    let explicit = if fip.ip.is_empty() {
        None
    } else {
        Some(fip.ip.clone())
    };
    let mut module = state.module.lock().await;
    module
        .allocate_floating_ip(&ctx.tenant_id, &mut fip, explicit)
        .await?;
    Ok((StatusCode::CREATED, Json(fip)))
}

async fn get_floating_ip(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<Json<FloatingIp>, AppError> {
    let module = state.module.lock().await;
    let fip = match module.get_floating_ip(&ctx.tenant_id, &name).await {
        Ok(f) => f,
        Err(_) => module
            .list_floating_ips(&ctx.tenant_id)
            .await?
            .into_iter()
            .find(|f| f.name == name)
            .ok_or_else(|| AppError::NotFound(format!("floating ip {name} not found")))?,
    };
    Ok(Json(fip))
}

async fn deallocate_floating_ip(
    State(state): State<NetworkApiState>,
    ctx: TenantContext,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut module = state.module.lock().await;
    module
        .deallocate_floating_ip(&ctx.tenant_id, &name)
        .await?;
    Ok(StatusCode::OK)
}
