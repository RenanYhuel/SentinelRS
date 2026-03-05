use std::path::Path;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::discovery::{scan_plugins_dir, DiscoveredPlugin};
use super::runtime::PluginRuntime;
use crate::config::PluginConfig;
use sentinel_common::proto::Metric;

pub struct PluginScheduler {
    config: PluginConfig,
    runtimes: Vec<LoadedPlugin>,
}

struct LoadedPlugin {
    name: String,
    runtime: PluginRuntime,
}

pub struct PluginSchedulerHandle {
    handle: JoinHandle<()>,
}

impl PluginSchedulerHandle {
    pub fn abort(&self) {
        self.handle.abort();
    }
}

impl PluginScheduler {
    pub fn new(config: PluginConfig) -> Self {
        Self {
            config,
            runtimes: Vec::new(),
        }
    }

    pub fn discover(&mut self) {
        let dir = Path::new(&self.config.dir);
        let signing_key = self.config.signing_key.as_ref().map(|k| k.as_bytes());

        let plugins = scan_plugins_dir(dir, signing_key);

        for DiscoveredPlugin {
            name,
            manifest,
            wasm_bytes,
            ..
        } in plugins
        {
            match PluginRuntime::load(&wasm_bytes, manifest) {
                Ok(runtime) => {
                    tracing::info!(target: "plugin", name = %name, "Plugin loaded");
                    self.runtimes.push(LoadedPlugin { name, runtime });
                }
                Err(e) => {
                    tracing::warn!(target: "plugin", name = %name, error = %e, "Failed to compile plugin");
                }
            }
        }

        tracing::info!(
            target: "plugin",
            count = self.runtimes.len(),
            "Plugin scheduler ready"
        );
    }

    pub fn loaded_count(&self) -> usize {
        self.runtimes.len()
    }

    pub fn spawn(self, tx: mpsc::Sender<Vec<Metric>>) -> PluginSchedulerHandle {
        let interval = Duration::from_secs(self.config.interval_seconds);
        let handle = tokio::spawn(async move {
            if self.runtimes.is_empty() {
                tracing::debug!(target: "plugin", "No plugins loaded, scheduler idle");
                return;
            }

            loop {
                let mut all_metrics = Vec::new();

                for loaded in &self.runtimes {
                    match loaded.runtime.execute() {
                        Ok(result) => {
                            for log_line in &result.logs {
                                tracing::debug!(
                                    target: "plugin",
                                    plugin = %loaded.name,
                                    "{log_line}"
                                );
                            }

                            let metrics = parse_plugin_metrics(&loaded.name, &result.metrics_json);
                            all_metrics.extend(metrics);
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "plugin",
                                plugin = %loaded.name,
                                error = %e,
                                "Plugin execution failed"
                            );
                        }
                    }
                }

                if !all_metrics.is_empty() {
                    tracing::debug!(
                        target: "plugin",
                        count = all_metrics.len(),
                        "Collected plugin metrics"
                    );
                    if tx.send(all_metrics).await.is_err() {
                        tracing::warn!(target: "plugin", "Metrics channel closed");
                        break;
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });

        PluginSchedulerHandle { handle }
    }
}

fn parse_plugin_metrics(plugin_name: &str, json_strings: &[String]) -> Vec<Metric> {
    let mut metrics = Vec::new();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    for json_str in json_strings {
        let trimmed = json_str.trim_matches('\0').trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(val) => {
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("plugin.unknown");

                let value = val.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);

                let mut labels = std::collections::HashMap::new();
                labels.insert("plugin".to_string(), plugin_name.to_string());

                if let Some(obj) = val.get("labels").and_then(|v| v.as_object()) {
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            labels.insert(k.clone(), s.to_string());
                        }
                    }
                }

                metrics.push(Metric {
                    name: format!("plugin.{name}"),
                    labels,
                    rtype: 1,
                    value: Some(sentinel_common::proto::metric::Value::ValueDouble(value)),
                    timestamp_ms: now_ms,
                });
            }
            Err(e) => {
                tracing::warn!(
                    target: "plugin",
                    plugin = %plugin_name,
                    error = %e,
                    "Invalid metric JSON"
                );
            }
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_metric_json() {
        let json = vec![r#"{"name":"cpu","value":42.5}"#.to_string()];
        let metrics = parse_plugin_metrics("test_plugin", &json);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "plugin.cpu");
        assert_eq!(metrics[0].labels.get("plugin").unwrap(), "test_plugin");
    }

    #[test]
    fn parse_metric_with_labels() {
        let json =
            vec![r#"{"name":"req_count","value":100,"labels":{"host":"web01"}}"#.to_string()];
        let metrics = parse_plugin_metrics("nginx", &json);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].labels.get("host").unwrap(), "web01");
        assert_eq!(metrics[0].labels.get("plugin").unwrap(), "nginx");
    }

    #[test]
    fn parse_invalid_json_skipped() {
        let json = vec![
            "not json".to_string(),
            r#"{"name":"ok","value":1}"#.to_string(),
        ];
        let metrics = parse_plugin_metrics("test", &json);
        assert_eq!(metrics.len(), 1);
    }

    #[test]
    fn empty_json_returns_empty() {
        let metrics = parse_plugin_metrics("test", &[]);
        assert!(metrics.is_empty());
    }

    #[test]
    fn scheduler_new_empty() {
        let config = PluginConfig {
            enabled: true,
            dir: "/tmp/nonexistent".into(),
            interval_seconds: 30,
            signing_key: None,
        };
        let sched = PluginScheduler::new(config);
        assert_eq!(sched.loaded_count(), 0);
    }
}
