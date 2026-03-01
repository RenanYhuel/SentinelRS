use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::jitter::apply_jitter;
use crate::collector::Collector;
use sentinel_common::proto::Metric;

pub struct ScheduledTask {
    pub interval: Duration,
    pub jitter_fraction: f64,
    pub collector: Arc<dyn Collector>,
}

pub struct TaskHandle {
    handle: JoinHandle<()>,
}

impl TaskHandle {
    pub fn abort(&self) {
        self.handle.abort();
    }
}

impl ScheduledTask {
    pub fn spawn(self, tx: mpsc::Sender<Vec<Metric>>) -> TaskHandle {
        let handle = tokio::spawn(async move {
            loop {
                let wait = apply_jitter(self.interval, self.jitter_fraction);
                tokio::time::sleep(wait).await;
                let metrics = self.collector.collect();
                if !metrics.is_empty() {
                    let _ = tx.send(metrics).await;
                }
            }
        });
        TaskHandle { handle }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_common::proto::Metric;

    struct FakeCollector;

    impl Collector for FakeCollector {
        fn collect(&self) -> Vec<Metric> {
            vec![Metric {
                name: "test.metric".into(),
                labels: Default::default(),
                rtype: 1,
                value: Some(sentinel_common::proto::metric::Value::ValueDouble(1.0)),
                timestamp_ms: 0,
            }]
        }
    }

    #[tokio::test]
    async fn scheduler_fires_and_sends_metrics() {
        let (tx, mut rx) = mpsc::channel(16);
        let task = ScheduledTask {
            interval: Duration::from_millis(50),
            jitter_fraction: 0.0,
            collector: Arc::new(FakeCollector),
        };
        let handle = task.spawn(tx);

        let metrics = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");

        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "test.metric");
        handle.abort();
    }
}
