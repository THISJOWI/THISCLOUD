//! T5.1 metrics model: Prometheus metric representation + text-format renderer.
//!
//! The renderer emits the Prometheus text exposition format (version 0.0.4):
//! one `# HELP` + `# TYPE` pair per metric name, then one sample line per
//! distinct label-set. Label keys are emitted sorted (BTreeMap order), samples
//! without labels are emitted as a bare name.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Prometheus metric family type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    #[default]
    Counter,
    Gauge,
}

impl MetricType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
        }
    }
}

/// A single metric sample (name + value + label-set + family type).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub metric_type: MetricType,
}

/// Render a float the way Prometheus expects: shortest round-tripping decimal,
/// trailing zeros trimmed (`1.0` → `1`, `1.50` → `1.5`), with the special
/// tokens `NaN`, `+Inf`, `-Inf` for non-finite values.
pub fn format_value(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "+Inf".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Inf".to_string()
    } else {
        value.to_string()
    }
}

/// Render metrics in the Prometheus text exposition format.
///
/// Names are emitted in first-seen order; within a name, one sample line per
/// distinct label-set (first occurrence wins). Label keys are sorted by key.
pub fn render_prometheus(metrics: &[Metric]) -> String {
    let mut out = String::new();

    // Preserve first-seen name order for deterministic blob output.
    let mut names: Vec<&str> = Vec::new();
    for m in metrics {
        if !names.contains(&m.name.as_str()) {
            names.push(m.name.as_str());
        }
    }

    for name in names {
        let type_str = metrics
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.metric_type.as_str())
            .unwrap_or("gauge");
        out.push_str(&format!("# HELP {name} {name}\n"));
        out.push_str(&format!("# TYPE {name} {type_str}\n"));

        let mut seen: Vec<&BTreeMap<String, String>> = Vec::new();
        for m in metrics.iter().filter(|m| m.name == name) {
            if seen.contains(&&m.labels) {
                continue;
            }
            seen.push(&m.labels);
            out.push_str(&format_sample(name, m));
        }
    }
    out
}

fn format_sample(name: &str, m: &Metric) -> String {
    if m.labels.is_empty() {
        format!("{} {}\n", name, format_value(m.value))
    } else {
        let labels = m
            .labels
            .iter()
            .map(|(k, v)| format!("{k}=\"{}\"", escape_label_value(v)))
            .collect::<Vec<_>>()
            .join(",");
        format!("{name}{{{labels}}} {}\n", format_value(m.value))
    }
}

/// Escape a label value per the Prometheus text format: backslash, double
/// quote and newline are backslash-escaped.
fn escape_label_value(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}