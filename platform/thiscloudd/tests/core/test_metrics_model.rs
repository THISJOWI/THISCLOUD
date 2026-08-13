//! T5.1 model tests: Prometheus text-format rendering + value formatting.

use crate::model::{format_value, render_prometheus, Metric, MetricType};

fn metric(name: &str, value: f64, labels: &[(&str, &str)], ty: MetricType) -> Metric {
    Metric {
        name: name.to_string(),
        value,
        labels: labels
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        metric_type: ty,
    }
}

#[test]
fn format_value_trims_trailing_zeros() {
    assert_eq!(format_value(1.0), "1");
    assert_eq!(format_value(1.50), "1.5");
    assert_eq!(format_value(0.1), "0.1");
    assert_eq!(format_value(42.0), "42");
    assert_eq!(format_value(1.23456), "1.23456");
}

#[test]
fn format_value_handles_special_floats() {
    assert_eq!(format_value(f64::NAN), "NaN");
    assert_eq!(format_value(f64::INFINITY), "+Inf");
    assert_eq!(format_value(f64::NEG_INFINITY), "-Inf");
}

#[test]
fn render_emits_help_and_type_once_per_name() {
    let metrics = vec![
        metric("thiscloud_vms", 3.0, &[], MetricType::Gauge),
        metric("thiscloud_vms", 4.0, &[("tenant", "a")], MetricType::Gauge),
    ];
    let out = render_prometheus(&metrics);
    assert_eq!(out.matches("# HELP thiscloud_vms thiscloud_vms").count(), 1);
    assert_eq!(out.matches("# TYPE thiscloud_vms gauge").count(), 1);
}

#[test]
fn render_type_line_reflects_metric_type() {
    let metrics = vec![metric(
        "thiscloud_api_requests_total",
        5.0,
        &[],
        MetricType::Counter,
    )];
    let out = render_prometheus(&metrics);
    assert!(out.contains("# TYPE thiscloud_api_requests_total counter"));
}

#[test]
fn render_labels_sorted_by_key() {
    let metrics = vec![metric(
        "thiscloud_vms",
        1.0,
        &[("zone", "z1"), ("tenant", "t1"), ("arch", "x86")],
        MetricType::Gauge,
    )];
    let out = render_prometheus(&metrics);
    let line = out
        .lines()
        .find(|l| l.starts_with("thiscloud_vms{"))
        .unwrap();
    let labels_part = line.split('{').nth(1).unwrap().trim_end_matches('}');
    let keys: Vec<&str> = labels_part
        .split(',')
        .map(|kv| kv.split('=').next().unwrap())
        .collect();
    assert_eq!(keys, vec!["arch", "tenant", "zone"]);
}

#[test]
fn render_sample_without_labels_is_bare_name() {
    let metrics = vec![metric("thiscloud_vms", 2.0, &[], MetricType::Gauge)];
    let out = render_prometheus(&metrics);
    assert!(out.contains("thiscloud_vms 2\n"));
    assert!(!out.contains("thiscloud_vms{"));
}

#[test]
fn render_emits_one_sample_per_distinct_label_set() {
    let metrics = vec![
        metric("thiscloud_vms", 1.0, &[("tenant", "a")], MetricType::Gauge),
        metric("thiscloud_vms", 5.0, &[("tenant", "a")], MetricType::Gauge),
        metric("thiscloud_vms", 2.0, &[("tenant", "b")], MetricType::Gauge),
    ];
    let out = render_prometheus(&metrics);
    assert_eq!(out.matches("thiscloud_vms{").count(), 2);
}

#[test]
fn render_blob_ordering_follows_first_seen_name_order() {
    let metrics = vec![
        metric("thiscloud_networks", 1.0, &[], MetricType::Gauge),
        metric("thiscloud_vms", 2.0, &[], MetricType::Gauge),
        metric("thiscloud_networks", 3.0, &[("tenant", "a")], MetricType::Gauge),
    ];
    let out = render_prometheus(&metrics);
    let pos_networks = out.find("# HELP thiscloud_networks").unwrap();
    let pos_vms = out.find("# HELP thiscloud_vms").unwrap();
    assert!(pos_networks < pos_vms);
}

#[test]
fn render_escapes_label_values() {
    let metrics = vec![metric(
        "thiscloud_vms",
        1.0,
        &[("name", "vm\"1\\x\ny")],
        MetricType::Gauge,
    )];
    let out = render_prometheus(&metrics);
    assert!(out.contains("name=\"vm\\\"1\\\\x\\ny\""));
}

#[test]
fn render_empty_input_is_empty_string() {
    assert_eq!(render_prometheus(&[]), "");
}

#[test]
fn metric_type_serde_lowercase() {
    let m: Metric =
        serde_json::from_value(serde_json::json!({"name": "x", "value": 1.0, "metric_type": "gauge"}))
            .unwrap();
    assert_eq!(m.metric_type, MetricType::Gauge);

    let m: Metric = serde_json::from_value(serde_json::json!({
        "name": "x",
        "value": 1.0,
        "metric_type": "counter"
    }))
    .unwrap();
    assert_eq!(m.metric_type, MetricType::Counter);
}

#[test]
fn metric_serde_defaults_labels_and_type() {
    let m: Metric =
        serde_json::from_value(serde_json::json!({"name": "x", "value": 1.0})).unwrap();
    assert!(m.labels.is_empty());
    assert_eq!(m.metric_type, MetricType::Counter);
}