use std::collections::HashMap;
use sysinfo::{Disks, Networks, System};

use super::naming::normalize_name;
use super::traits::Collector;
use sentinel_common::proto::{metric::Value, Metric, MetricType};

pub struct SystemCollector {
    sys: System,
    disks: Disks,
    networks: Networks,
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCollector {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
        }
    }

    fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    fn gauge(name: &str, value: f64, labels: HashMap<String, String>) -> Metric {
        Metric {
            name: normalize_name(name),
            labels,
            rtype: MetricType::Gauge as i32,
            value: Some(Value::ValueDouble(value)),
            timestamp_ms: Self::now_ms(),
        }
    }

    fn counter(name: &str, value: f64, labels: HashMap<String, String>) -> Metric {
        Metric {
            name: normalize_name(name),
            labels,
            rtype: MetricType::Counter as i32,
            value: Some(Value::ValueDouble(value)),
            timestamp_ms: Self::now_ms(),
        }
    }

    fn collect_cpu(&self) -> Vec<Metric> {
        self.sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| {
                let mut labels = HashMap::new();
                labels.insert("core".into(), i.to_string());
                Self::gauge(
                    &format!("cpu.core.{}.usage_percent", i),
                    cpu.cpu_usage() as f64,
                    labels,
                )
            })
            .collect()
    }

    fn collect_memory(&self) -> Vec<Metric> {
        let empty = HashMap::new();
        vec![
            Self::gauge(
                "mem.total_bytes",
                self.sys.total_memory() as f64,
                empty.clone(),
            ),
            Self::gauge(
                "mem.used_bytes",
                self.sys.used_memory() as f64,
                empty.clone(),
            ),
            Self::gauge(
                "mem.available_bytes",
                self.sys.available_memory() as f64,
                empty.clone(),
            ),
            Self::gauge(
                "mem.swap_total_bytes",
                self.sys.total_swap() as f64,
                empty.clone(),
            ),
            Self::gauge("mem.swap_used_bytes", self.sys.used_swap() as f64, empty),
        ]
    }

    fn collect_disks(&self) -> Vec<Metric> {
        self.disks
            .iter()
            .flat_map(|disk| {
                let name = disk.name().to_string_lossy().to_string();
                let dev = if name.is_empty() {
                    disk.mount_point()
                        .to_string_lossy()
                        .replace(['\\', '/'], "_")
                } else {
                    name
                };
                let mut labels = HashMap::new();
                labels.insert("device".into(), dev.clone());
                vec![
                    Self::gauge(
                        &format!("disk.{}.total_bytes", dev),
                        disk.total_space() as f64,
                        labels.clone(),
                    ),
                    Self::gauge(
                        &format!("disk.{}.available_bytes", dev),
                        disk.available_space() as f64,
                        labels,
                    ),
                ]
            })
            .collect()
    }

    fn collect_network(&self) -> Vec<Metric> {
        self.networks
            .iter()
            .flat_map(|(iface, data)| {
                let mut labels = HashMap::new();
                labels.insert("interface".into(), iface.clone());
                vec![
                    Self::counter(
                        &format!("net.{}.bytes_recv", iface),
                        data.total_received() as f64,
                        labels.clone(),
                    ),
                    Self::counter(
                        &format!("net.{}.bytes_sent", iface),
                        data.total_transmitted() as f64,
                        labels,
                    ),
                ]
            })
            .collect()
    }

    fn collect_uptime(&self) -> Vec<Metric> {
        vec![Self::gauge(
            "uptime_seconds",
            System::uptime() as f64,
            HashMap::new(),
        )]
    }

    fn collect_processes(&self) -> Vec<Metric> {
        let total = self.sys.processes().len();
        vec![Self::gauge(
            "process.count_total",
            total as f64,
            HashMap::new(),
        )]
    }
}

impl Collector for SystemCollector {
    fn collect(&self) -> Vec<Metric> {
        let mut metrics = Vec::new();
        metrics.extend(self.collect_cpu());
        metrics.extend(self.collect_memory());
        metrics.extend(self.collect_disks());
        metrics.extend(self.collect_network());
        metrics.extend(self.collect_uptime());
        metrics.extend(self.collect_processes());
        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_non_empty_metrics() {
        let collector = SystemCollector::new();
        let metrics = collector.collect();
        assert!(!metrics.is_empty(), "must produce at least one metric");
    }

    #[test]
    fn cpu_metrics_have_core_label() {
        let collector = SystemCollector::new();
        let metrics = collector.collect();
        let cpu_metrics: Vec<_> = metrics
            .iter()
            .filter(|m| m.name.starts_with("cpu.core"))
            .collect();
        for m in &cpu_metrics {
            assert!(m.labels.contains_key("core"));
        }
    }

    #[test]
    fn memory_metrics_present() {
        let collector = SystemCollector::new();
        let metrics = collector.collect();
        let mem_names: Vec<_> = metrics
            .iter()
            .filter(|m| m.name.starts_with("mem."))
            .map(|m| m.name.as_str())
            .collect();
        assert!(mem_names.contains(&"mem.total_bytes"));
        assert!(mem_names.contains(&"mem.used_bytes"));
    }
}
