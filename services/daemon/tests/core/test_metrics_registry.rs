//! T5.1 registry tests: set/increment semantics, snapshot isolation.

use crate::model::{Metric, MetricType};
use crate::module::MetricsModule;
use crate::registry::MetricRegistry;
use std::collections::BTreeMap;
use std::sync::Arc;

fn labels(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn register_adds_metric() {
    let reg = MetricRegistry::new();
    reg.register(Metric {
        name: "thiscloud_vms".into(),
        value: 1.0,
        labels: labels(&[]),
        metric_type: MetricType::Gauge,
    });
    assert_eq!(reg.snapshot().len(), 1);
}

#[test]
fn set_gauge_creates_when_absent() {
    let reg = MetricRegistry::new();
    reg.set_gauge("thiscloud_vms", 2.0, labels(&[("tenant", "a")]));
    let snap = reg.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].value, 2.0);
    assert_eq!(snap[0].metric_type, MetricType::Gauge);
    assert_eq!(snap[0].name, "thiscloud_vms");
}

#[test]
fn set_gauge_overwrites_existing_value() {
    let reg = MetricRegistry::new();
    reg.set_gauge("thiscloud_vms", 2.0, labels(&[("tenant", "a")]));
    reg.set_gauge("thiscloud_vms", 9.0, labels(&[("tenant", "a")]));
    let snap = reg.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].value, 9.0);
}

#[test]
fn set_gauge_distinguishes_label_sets() {
    let reg = MetricRegistry::new();
    reg.set_gauge("thiscloud_vms", 1.0, labels(&[("tenant", "a")]));
    reg.set_gauge("thiscloud_vms", 2.0, labels(&[("tenant", "b")]));
    assert_eq!(reg.snapshot().len(), 2);
}

#[test]
fn inc_counter_creates_with_delta() {
    let reg = MetricRegistry::new();
    reg.inc_counter("thiscloud_api_requests_total", 5.0, labels(&[]));
    let snap = reg.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].value, 5.0);
    assert_eq!(snap[0].metric_type, MetricType::Counter);
}

#[test]
fn inc_counter_accumulates() {
    let reg = MetricRegistry::new();
    reg.inc_counter("thiscloud_api_requests_total", 5.0, labels(&[]));
    reg.inc_counter("thiscloud_api_requests_total", 3.0, labels(&[]));
    let snap = reg.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].value, 8.0);
}

#[test]
fn inc_counter_does_not_merge_gauge() {
    let reg = MetricRegistry::new();
    reg.set_gauge("thiscloud_vms", 1.0, labels(&[]));
    reg.inc_counter("thiscloud_vms", 2.0, labels(&[]));
    let snap = reg.snapshot();
    assert_eq!(snap.len(), 2);
}

#[test]
fn set_gauge_does_not_clobber_counter() {
    let reg = MetricRegistry::new();
    reg.inc_counter("thiscloud_api_requests_total", 5.0, labels(&[]));
    reg.set_gauge("thiscloud_api_requests_total", 99.0, labels(&[]));
    let snap = reg.snapshot();
    assert_eq!(snap.len(), 2);
    let counter = snap
        .iter()
        .find(|m| m.metric_type == MetricType::Counter)
        .unwrap();
    assert_eq!(counter.value, 5.0);
}

#[test]
fn snapshot_is_independent_clone() {
    let reg = MetricRegistry::new();
    reg.set_gauge("thiscloud_vms", 1.0, labels(&[]));
    let snap = reg.snapshot();
    reg.clear();
    assert_eq!(snap.len(), 1);
    assert!(reg.snapshot().is_empty());
}

#[test]
fn clear_empties_registry() {
    let reg = MetricRegistry::new();
    reg.set_gauge("thiscloud_vms", 1.0, labels(&[]));
    reg.inc_counter("thiscloud_api_requests_total", 1.0, labels(&[]));
    reg.clear();
    assert!(reg.snapshot().is_empty());
}

#[test]
fn metrics_module_collect_snapshots_registry() {
    let registry = Arc::new(MetricRegistry::new());
    registry.set_gauge("thiscloud_vms", 3.0, labels(&[]));
    let module = MetricsModule::new(registry.clone());
    let collected = module.collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].value, 3.0);
    // Module shares the same registry instance.
    registry.set_gauge("thiscloud_vms", 4.0, labels(&[]));
    assert_eq!(module.collect()[0].value, 4.0);
}