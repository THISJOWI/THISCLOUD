//! T5.1 metrics module: exposes the shared registry to the daemon's module
//! lifecycle. No store — metrics are ephemeral snapshots.

use crate::core::Module;
use super::model::Metric;
use super::registry::MetricRegistry;
use async_trait::async_trait;
use std::sync::Arc;

pub struct MetricsModule {
    registry: Arc<MetricRegistry>,
}

impl MetricsModule {
    pub fn new(registry: Arc<MetricRegistry>) -> Self {
        Self { registry }
    }

    /// Snapshot of all currently registered metrics.
    pub fn collect(&self) -> Vec<Metric> {
        self.registry.snapshot()
    }
}

#[async_trait]
impl Module for MetricsModule {
    fn name(&self) -> &str {
        "metrics"
    }

    async fn start(&mut self, _event_bus: &crate::core::EventBus) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_running(&self) -> bool {
        true
    }
}