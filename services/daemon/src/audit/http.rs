use crate::audit::middleware::AuditState;
use crate::audit::model::{AuditAction, AuditEntry, AuditFilter};
use crate::core::AppError;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};

#[derive(serde::Deserialize, Default)]
pub struct AuditQuery {
    pub tenant_id: Option<String>,
    pub user: Option<String>,
    pub action: Option<String>,
    pub resource: Option<String>,
    pub limit: Option<usize>,
}

pub fn app(state: AuditState) -> Router {
    Router::new()
        .route("/audit", get(query_audit))
        .route("/audit/export", get(export_audit))
        .with_state(state)
}

async fn query_audit(
    State(state): State<AuditState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEntry>>, AppError> {
    let action = q.action.as_deref().and_then(parse_action);
    let filter = AuditFilter {
        tenant_id: q.tenant_id,
        user: q.user,
        action,
        resource: q.resource,
        limit: q.limit,
    };
    let store = state.store.lock().await;
    let entries = store.query(&filter).await;
    Ok(Json(entries))
}

/// Full append-only audit log as a JSON export (unfiltered).
async fn export_audit(State(state): State<AuditState>) -> Result<Json<Vec<AuditEntry>>, AppError> {
    let store = state.store.lock().await;
    let entries = store
        .query(&AuditFilter {
            limit: None,
            ..Default::default()
        })
        .await;
    Ok(Json(entries))
}

fn parse_action(s: &str) -> Option<AuditAction> {
    match s.to_lowercase().as_str() {
        "create" | "post" => Some(AuditAction::Create),
        "read" | "get" => Some(AuditAction::Read),
        "update" | "put" | "patch" => Some(AuditAction::Update),
        "delete" | "del" => Some(AuditAction::Delete),
        _ => None,
    }
}