//! T5.1 metrics registry: in-memory, ephemeral metric store.
//!
//! Gauges are set (overwrite), counters are incremented (accumulate). Both
//! key on (name, label-set, type) so a gauge never clobbers a counter with the
//! same name and vice versa. No persistence — snapshots are ephemeral.

use super::model::{Metric, MetricType};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Thread-safe registry of currently registered metrics.
#[derive(Debug, Default)]
pub struct MetricRegistry {
    metrics: Mutex<Vec<Metric>>,
}

impl MetricRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a metric verbatim (append). Callers that want set/increment
    /// semantics should use [`Self::set_gauge`] / [`Self::inc_counter`].
    pub fn register(&self, metric: Metric) {
        self.metrics.lock().unwrap().push(metric);
    }

    /// Set a gauge value, overwriting any existing gauge with the same
    /// name + label-set.
    pub fn set_gauge(&self, name: &str, value: f64, labels: BTreeMap<String, String>) {
        let mut metrics = self.metrics.lock().unwrap();
        if let Some(m) = metrics.iter_mut().find(|m| {
            m.name == name && m.metric_type == MetricType::Gauge && m.labels == labels
        }) {
            m.value = value;
        } else {
            metrics.push(Metric {
                name: name.to_string(),
                value,
                labels,
                metric_type: MetricType::Gauge,
            });
        }
    }

    /// Increment a counter by `delta`, creating it (with value `delta`) when
    /// no counter with the same name + label-set exists yet.
    pub fn inc_counter(&self, name: &str, delta: f64, labels: BTreeMap<String, String>) {
        let mut metrics = self.metrics.lock().unwrap();
        if let Some(m) = metrics.iter_mut().find(|m| {
            m.name == name && m.metric_type == MetricType::Counter && m.labels == labels
        }) {
            m.value += delta;
        } else {
            metrics.push(Metric {
                name: name.to_string(),
                value: delta,
                labels,
                metric_type: MetricType::Counter,
            });
        }
    }

    /// Snapshot of all registered metrics (independent clone).
    pub fn snapshot(&self) -> Vec<Metric> {
        self.metrics.lock().unwrap().clone()
    }

    /// Drop all registered metrics.
    pub fn clear(&self) {
        self.metrics.lock().unwrap().clear();
    }
}