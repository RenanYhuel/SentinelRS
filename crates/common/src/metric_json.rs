use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::proto;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricJson {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub metric_type: String,
    pub value: MetricValueJson,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MetricValueJson {
    Double(f64),
    Int(i64),
    Histogram {
        boundaries: Vec<f64>,
        counts: Vec<u64>,
        count: u64,
        sum: f64,
    },
}

impl From<&proto::Metric> for MetricJson {
    fn from(m: &proto::Metric) -> Self {
        let metric_type = match proto::MetricType::try_from(m.rtype) {
            Ok(proto::MetricType::Gauge) => "gauge",
            Ok(proto::MetricType::Counter) => "counter",
            Ok(proto::MetricType::Histogram) => "histogram",
            _ => "unspecified",
        }
        .to_string();

        let value = match &m.value {
            Some(proto::metric::Value::ValueDouble(v)) => MetricValueJson::Double(*v),
            Some(proto::metric::Value::ValueInt(v)) => MetricValueJson::Int(*v),
            Some(proto::metric::Value::Histogram(h)) => MetricValueJson::Histogram {
                boundaries: h.boundaries.clone(),
                counts: h.counts.clone(),
                count: h.count,
                sum: h.sum,
            },
            None => MetricValueJson::Double(0.0),
        };

        MetricJson {
            name: m.name.clone(),
            labels: m.labels.clone(),
            metric_type,
            value,
            timestamp_ms: m.timestamp_ms,
        }
    }
}

impl From<&MetricJson> for proto::Metric {
    fn from(j: &MetricJson) -> Self {
        let rtype = match j.metric_type.as_str() {
            "gauge" => proto::MetricType::Gauge as i32,
            "counter" => proto::MetricType::Counter as i32,
            "histogram" => proto::MetricType::Histogram as i32,
            _ => proto::MetricType::Unspecified as i32,
        };

        let value = match &j.value {
            MetricValueJson::Double(v) => Some(proto::metric::Value::ValueDouble(*v)),
            MetricValueJson::Int(v) => Some(proto::metric::Value::ValueInt(*v)),
            MetricValueJson::Histogram {
                boundaries,
                counts,
                count,
                sum,
            } => Some(proto::metric::Value::Histogram(proto::Histogram {
                boundaries: boundaries.clone(),
                counts: counts.clone(),
                count: *count,
                sum: *sum,
            })),
        };

        proto::Metric {
            name: j.name.clone(),
            labels: j.labels.clone(),
            rtype,
            value,
            timestamp_ms: j.timestamp_ms,
        }
    }
}

pub fn to_json(metric: &proto::Metric) -> serde_json::Result<String> {
    let mj = MetricJson::from(metric);
    serde_json::to_string(&mj)
}

pub fn from_json(json: &str) -> serde_json::Result<proto::Metric> {
    let mj: MetricJson = serde_json::from_str(json)?;
    Ok(proto::Metric::from(&mj))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_gauge() -> proto::Metric {
        proto::Metric {
            name: "cpu.usage".into(),
            labels: {
                let mut m = HashMap::new();
                m.insert("core".into(), "0".into());
                m
            },
            rtype: proto::MetricType::Gauge as i32,
            value: Some(proto::metric::Value::ValueDouble(55.5)),
            timestamp_ms: 1709251200000,
        }
    }

    #[test]
    fn roundtrip_gauge() {
        let original = sample_gauge();
        let json_str = to_json(&original).unwrap();
        let restored = from_json(&json_str).unwrap();
        assert_eq!(original.name, restored.name);
        assert_eq!(original.rtype, restored.rtype);
        assert_eq!(original.timestamp_ms, restored.timestamp_ms);
        match (&original.value, &restored.value) {
            (
                Some(proto::metric::Value::ValueDouble(a)),
                Some(proto::metric::Value::ValueDouble(b)),
            ) => assert!((a - b).abs() < f64::EPSILON),
            _ => panic!("value mismatch"),
        }
    }

    #[test]
    fn roundtrip_histogram() {
        let metric = proto::Metric {
            name: "request.latency".into(),
            labels: HashMap::new(),
            rtype: proto::MetricType::Histogram as i32,
            value: Some(proto::metric::Value::Histogram(proto::Histogram {
                boundaries: vec![10.0, 50.0, 100.0],
                counts: vec![5, 10, 3],
                count: 18,
                sum: 850.0,
            })),
            timestamp_ms: 1709251200000,
        };
        let json_str = to_json(&metric).unwrap();
        let restored = from_json(&json_str).unwrap();
        assert_eq!(metric.name, restored.name);
    }
}
