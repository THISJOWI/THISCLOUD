//! T5.1 Prometheus metrics + Grafana observability module.

pub mod http;
pub mod model;
pub mod module;
pub mod registry;

pub use http::{app, MetricsApiState};
pub use model::{format_value, render_prometheus, Metric, MetricType};
pub use module::MetricsModule;
pub use registry::MetricRegistry;