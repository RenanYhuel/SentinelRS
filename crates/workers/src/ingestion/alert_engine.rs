use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::aggregator::AggregatorStore;
use crate::alert::{AlertStore, Evaluator};
use crate::notifier::dispatcher::Dispatcher;
use crate::storage::RuleLoader;
use crate::transform::MetricRow;

const AGGREGATOR_WINDOW_MS: i64 = 120_000;
const RULE_RELOAD_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

pub struct AlertEngine {
    evaluator: RwLock<Evaluator>,
    aggregator: AggregatorStore,
    alert_store: AlertStore,
    rule_loader: RuleLoader,
    dispatcher: Dispatcher,
}

impl AlertEngine {
    pub async fn new(pool: PgPool) -> Result<Self, sqlx::Error> {
        let rule_loader = RuleLoader::new(pool.clone());
        let rules = rule_loader.load_enabled().await.unwrap_or_default();

        let count = rules.len();
        tracing::info!(target: "alert", count, "Alert rules loaded");

        Ok(Self {
            evaluator: RwLock::new(Evaluator::new(rules)),
            aggregator: AggregatorStore::new(AGGREGATOR_WINDOW_MS),
            alert_store: AlertStore::new(pool.clone()),
            rule_loader,
            dispatcher: Dispatcher::new(pool),
        })
    }

    pub async fn process(&self, agent_id: &str, rows: &[MetricRow]) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        for row in rows {
            if let Some(value) = row.value {
                self.aggregator
                    .ingest(&row.agent_id, &row.name, row.time_ms, value);
            }
        }

        let evaluator = self.evaluator.read().await;
        let events = evaluator.evaluate(agent_id, &self.aggregator, now_ms);
        let nid_map: Vec<Vec<String>> = events
            .iter()
            .map(|e| evaluator.notifier_ids_for_rule(&e.rule_id).to_vec())
            .collect();
        drop(evaluator);

        for (event, nids) in events.iter().zip(nid_map.iter()) {
            tracing::info!(
                target: "alert",
                rule = %event.rule_name,
                agent = %event.agent_id,
                status = %event.status_str(),
                value = event.value,
                "Alert event"
            );
            if let Err(e) = self.alert_store.persist(event).await {
                tracing::error!(target: "alert", error = %e, "Failed to persist alert");
            }

            if !nids.is_empty() {
                self.dispatcher.dispatch(event, nids).await;
            }
        }
    }

    pub async fn reload_rules(&self) {
        match self.rule_loader.load_enabled().await {
            Ok(rules) => {
                let count = rules.len();
                self.evaluator.write().await.set_rules(rules);
                tracing::debug!(target: "alert", count, "Alert rules reloaded");
            }
            Err(e) => {
                tracing::error!(target: "alert", error = %e, "Failed to reload rules");
            }
        }
    }

    pub fn spawn_reload_loop(self: &Arc<Self>, cancel: tokio_util::sync::CancellationToken) {
        let engine = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(RULE_RELOAD_INTERVAL) => {
                        engine.reload_rules().await;
                    }
                }
            }
        });
    }
}
