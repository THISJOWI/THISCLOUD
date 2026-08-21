//! T5.1 metrics HTTP surface.
//!
//! Two routes:
//! - `GET /metrics` — public, anonymous Prometheus scrape. Deliberately NO
//!   `TenantContext` and NO `RequireRole` middleware: Prometheus pulls without
//!   tenant scope or auth.
//! - `POST /metrics/push` — tenant-scoped, write-role protected. Accepts a
//!   single `Metric` or an array; gauges are set, counters incremented.

use crate::auth::rbac::{RequireRole, WriteSet};
use crate::auth::TenantContext;
use crate::core::AppError;
use super::model::{Metric, MetricType};
use super::registry::MetricRegistry;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::sync::Arc;

#[derive(Clone)]
pub struct MetricsApiState {
    pub registry: Arc<MetricRegistry>,
}

impl MetricsApiState {
    pub fn new(registry: Arc<MetricRegistry>) -> Self {
        Self { registry }
    }
}

pub fn app(state: MetricsApiState) -> Router {
    // Public scrape endpoint: no TenantContext, no RBAC (anonymous pull).
    let public = Router::new().route("/metrics", get(render_metrics));

    // Push endpoint: tenant-scoped + write-role protected.
    let push = Router::new()
        .route("/metrics/push", post(push_metrics))
        .route_layer(middleware::from_extractor::<RequireRole<WriteSet>>());

    public.merge(push).with_state(state)
}

async fn render_metrics(State(state): State<MetricsApiState>) -> impl IntoResponse {
    let body = super::model::render_prometheus(&state.registry.snapshot());
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
}

/// Push payload: a single metric or a batch.
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum PushPayload {
    Single(Metric),
    Many(Vec<Metric>),
}

impl PushPayload {
    fn into_metrics(self) -> Vec<Metric> {
        match self {
            PushPayload::Single(m) => vec![m],
            PushPayload::Many(v) => v,
        }
    }
}

async fn push_metrics(
    State(state): State<MetricsApiState>,
    _ctx: TenantContext,
    Json(payload): Json<PushPayload>,
) -> Result<impl IntoResponse, AppError> {
    for metric in payload.into_metrics() {
        if metric.name.is_empty() {
            return Err(AppError::Validation("metric name is required".to_string()));
        }
        match metric.metric_type {
            MetricType::Counter => {
                state
                    .registry
                    .inc_counter(&metric.name, metric.value, metric.labels);
            }
            MetricType::Gauge => {
                state
                    .registry
                    .set_gauge(&metric.name, metric.value, metric.labels);
            }
        }
    }
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}