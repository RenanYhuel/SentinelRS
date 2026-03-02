use std::time::Duration;

use chrono::Utc;

use super::presence::{DisconnectReason, PresenceEvent, PresenceEventBus};
use super::registry::SessionRegistry;

const DEFAULT_CHECK_INTERVAL_MS: u64 = 5_000;
const STALE_MULTIPLIER: i64 = 3;
const EVICT_MULTIPLIER: i64 = 6;

pub struct WatchdogConfig {
    pub check_interval: Duration,
    pub stale_multiplier: i64,
    pub evict_multiplier: i64,
    pub heartbeat_interval_ms: i64,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_millis(DEFAULT_CHECK_INTERVAL_MS),
            stale_multiplier: STALE_MULTIPLIER,
            evict_multiplier: EVICT_MULTIPLIER,
            heartbeat_interval_ms: 10_000,
        }
    }
}

pub fn spawn_watchdog(
    registry: SessionRegistry,
    events: PresenceEventBus,
    config: WatchdogConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        run_watchdog(registry, events, config).await;
    })
}

async fn run_watchdog(registry: SessionRegistry, events: PresenceEventBus, config: WatchdogConfig) {
    let stale_threshold_ms = config.heartbeat_interval_ms * config.stale_multiplier;
    let evict_threshold_ms = config.heartbeat_interval_ms * config.evict_multiplier;

    tracing::info!(
        target: "conn",
        check_interval_ms = config.check_interval.as_millis() as u64,
        stale_after_ms = stale_threshold_ms,
        evict_after_ms = evict_threshold_ms,
        "Watchdog started"
    );

    let mut interval = tokio::time::interval(config.check_interval);

    loop {
        interval.tick().await;

        let stale_agents = registry.find_stale(stale_threshold_ms);
        for (agent_id, ms_ago) in &stale_agents {
            if *ms_ago < evict_threshold_ms {
                events.emit(PresenceEvent::AgentStale {
                    agent_id: agent_id.clone(),
                    last_ping_ms_ago: *ms_ago,
                    expected_interval_ms: config.heartbeat_interval_ms,
                    at: Utc::now(),
                });
                tracing::warn!(
                    target: "conn",
                    agent_id = %agent_id,
                    last_ping_ms_ago = ms_ago,
                    "Agent stale"
                );
            }
        }

        let evicted = registry.evict_stale(evict_threshold_ms);
        for agent_id in evicted {
            events.emit(PresenceEvent::AgentDisconnected {
                agent_id: agent_id.clone(),
                reason: DisconnectReason::StaleTimeout,
                connected_duration_ms: 0,
                at: Utc::now(),
            });
            tracing::error!(
                target: "conn",
                agent_id = %agent_id,
                "Agent evicted — connection lost (no heartbeat)"
            );
        }
    }
}
