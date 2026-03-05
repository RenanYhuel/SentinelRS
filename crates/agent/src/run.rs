use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

use crate::api::{self, AgentState};
use crate::batch::BatchComposer;
use crate::buffer::{compact, needs_compaction, Wal};
use crate::collector::SystemCollector;
use crate::config::AgentConfig;
use crate::exporter::{GrpcClient, RetryPolicy, SendLoop};
use crate::persistence::{AgentPersistedState, VolumeLayout};
use crate::plugin::PluginScheduler;
use crate::scheduler::ScheduledTask;
use crate::stream::StreamClient;
use sentinel_common::logging;

const COMPACTION_THRESHOLD_MB: u64 = 32;
const STATE_SAVE_INTERVAL_SECS: u64 = 60;

pub async fn run(config: AgentConfig, legacy_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    let agent_id = config
        .agent_id
        .clone()
        .unwrap_or_else(|| sysinfo::System::host_name().unwrap_or_else(|| "unknown".into()));

    let secret = resolve_secret(&config)?;

    let mode_label = if legacy_mode {
        "legacy (V1)"
    } else {
        "stream (V2)"
    };
    tracing::info!(
        target: "cfg",
        agent_id = %agent_id,
        server = %config.server,
        interval_s = config.collect.interval_seconds,
        mode = mode_label,
        "Agent configured"
    );

    let segment_bytes = config.buffer.segment_size_mb * 1024 * 1024;
    let sw = logging::stopwatch();
    let wal = Wal::open(Path::new(&config.buffer.wal_dir), true, segment_bytes)?;
    tracing::info!(target: "boot", "WAL opened{sw}");

    run_compaction_if_needed(&config.buffer.wal_dir, &wal)?;

    let resume_seq = wal.next_id();
    let pending = wal.unacked_count()?;

    if pending > 0 {
        tracing::info!(
            target: "boot",
            pending,
            resume_seq,
            "Resuming from previous state. {} pending batches in WAL",
            pending
        );
    }

    let wal = Arc::new(Mutex::new(wal));

    let state = AgentState::new();

    let config_parent = Path::new(&config.buffer.wal_dir)
        .parent()
        .unwrap_or_else(|| Path::new("/etc/sentinel"));
    let layout = VolumeLayout::new(config_parent);
    let persisted = load_or_create_persisted_state(&layout, &agent_id, &config.server, resume_seq)?;
    let persisted = Arc::new(Mutex::new(persisted));

    let (metrics_tx, metrics_rx) = mpsc::channel(256);

    spawn_collector(config.collect.interval_seconds, metrics_tx.clone());
    spawn_plugin_scheduler(&config, metrics_tx);
    spawn_batcher(agent_id.clone(), wal.clone(), metrics_rx, resume_seq);

    if legacy_mode {
        spawn_legacy_sender(
            config.server.clone(),
            agent_id.clone(),
            secret,
            wal.clone(),
            state.clone(),
        );
    } else {
        spawn_stream_sender(
            config.server.clone(),
            agent_id.clone(),
            secret,
            wal.clone(),
            state.clone(),
        );
    }

    spawn_api(config.api_port, state).await;
    spawn_state_saver(persisted.clone(), wal.clone(), layout.state_dir());

    tracing::info!(target: "system", agent_id = %agent_id, "Agent running");
    crate::shutdown::wait_for_shutdown().await;

    tracing::info!(target: "system", agent_id = %agent_id, "Shutting down");
    {
        let w = wal.lock().await;
        let _ = w.save_meta();
    }
    {
        let mut ps = persisted.lock().await;
        let w = wal.lock().await;
        ps.update_seq(w.next_id());
        ps.record_shutdown();
        let _ = ps.save(&layout.state_dir());
    }

    Ok(())
}

fn load_or_create_persisted_state(
    layout: &VolumeLayout,
    agent_id: &str,
    server_url: &str,
    resume_seq: u64,
) -> Result<AgentPersistedState, Box<dyn std::error::Error>> {
    let state_dir = layout.state_dir();
    let _ = std::fs::create_dir_all(&state_dir);

    let mut state = match AgentPersistedState::load(&state_dir)? {
        Some(mut s) => {
            let clean = s.clean_shutdown;
            if !clean {
                tracing::warn!(target: "boot", "Previous shutdown was not clean (crash detected)");
            }
            s.record_boot();
            s
        }
        None => {
            tracing::info!(target: "boot", "First boot, creating agent state");
            let mut s = AgentPersistedState::new(agent_id.into(), server_url.into());
            s.record_boot();
            s
        }
    };

    state.update_seq(resume_seq);
    state.save(&state_dir)?;

    tracing::info!(
        target: "boot",
        agent_id = %state.agent_id,
        boot_count = state.boot_count,
        seq = state.seq_counter,
        "Agent state loaded"
    );

    Ok(state)
}

fn run_compaction_if_needed(wal_dir: &str, wal: &Wal) -> Result<(), Box<dyn std::error::Error>> {
    let threshold = COMPACTION_THRESHOLD_MB * 1024 * 1024;
    let dir = Path::new(wal_dir);
    if needs_compaction(dir, threshold)? {
        let sw = logging::stopwatch();
        tracing::info!(target: "boot", "WAL compaction needed, running");
        let meta = wal.save_meta()?;
        compact(dir, &meta)?;
        tracing::info!(target: "boot", "WAL compaction complete{sw}");
    }
    Ok(())
}

fn spawn_collector(interval_secs: u64, tx: mpsc::Sender<Vec<sentinel_common::proto::Metric>>) {
    let collector = Arc::new(SystemCollector::new());
    let _handle = ScheduledTask {
        interval: Duration::from_secs(interval_secs),
        jitter_fraction: 0.1,
        collector,
    }
    .spawn(tx);
}

fn spawn_batcher(
    agent_id: String,
    wal: Arc<Mutex<Wal>>,
    mut rx: mpsc::Receiver<Vec<sentinel_common::proto::Metric>>,
    resume_seq: u64,
) {
    tokio::spawn(async move {
        let mut composer = BatchComposer::new(agent_id, resume_seq);
        while let Some(metrics) = rx.recv().await {
            let batch = composer.compose(metrics);
            let encoded = BatchComposer::encode_batch(&batch);
            let mut w = wal.lock().await;
            if let Err(e) = w.append(encoded) {
                tracing::error!(target: "data", error = %e, "WAL write failed");
            }
        }
    });
}

fn spawn_legacy_sender(
    server: String,
    agent_id: String,
    secret: Vec<u8>,
    wal: Arc<Mutex<Wal>>,
    state: AgentState,
) {
    tokio::spawn(async move {
        let send_loop = SendLoop {
            retry_policy: RetryPolicy::default().with_max_attempts(5),
        };

        loop {
            match GrpcClient::connect(&server, agent_id.clone(), &secret, None).await {
                Ok(mut client) => {
                    tracing::info!(target: "conn", server = %server, "Connected to server");
                    state.set_ready(true);

                    loop {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        let mut w = wal.lock().await;
                        match send_loop.send_pending(&mut w, &mut client).await {
                            Ok(n) if n > 0 => {
                                for _ in 0..n {
                                    state.increment_batches_sent();
                                }
                                tracing::debug!(target: "data", sent = n, "Batches sent");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(target: "conn", error = %e, "Send failed, will reconnect");
                                state.increment_batches_failed();
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(target: "conn", error = %e, server = %server, "Server unreachable, retrying in 10s");
                    state.set_ready(false);
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
}

fn spawn_stream_sender(
    server: String,
    agent_id: String,
    secret: Vec<u8>,
    wal: Arc<Mutex<Wal>>,
    state: AgentState,
) {
    tokio::spawn(async move {
        let version = env!("CARGO_PKG_VERSION").to_string();
        let key_id = "default".to_string();

        let client = StreamClient::new(
            server,
            agent_id.clone(),
            version,
            key_id,
            &secret,
            wal.clone(),
        );

        state.set_ready(true);
        client.run(None).await;
    });
}

async fn spawn_api(port: u16, state: AgentState) {
    let addr = format!("0.0.0.0:{port}");
    tokio::spawn(async move {
        match TcpListener::bind(&addr).await {
            Ok(listener) => {
                tracing::info!(target: "rest", addr = %addr, "HTTP API listening");
                if let Err(e) = api::serve(listener, state).await {
                    tracing::error!(target: "rest", error = %e, "HTTP API error");
                }
            }
            Err(e) => {
                tracing::error!(
                    target: "rest",
                    error = %e,
                    "{}",
                    sentinel_common::logging::actionable::port_in_use(&addr, &e)
                );
            }
        }
    });
}

fn spawn_state_saver(
    persisted: Arc<Mutex<AgentPersistedState>>,
    wal: Arc<Mutex<Wal>>,
    state_dir: std::path::PathBuf,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(STATE_SAVE_INTERVAL_SECS)).await;
            let w = wal.lock().await;
            let seq = w.next_id();
            let _ = w.save_meta();
            drop(w);

            let mut ps = persisted.lock().await;
            ps.update_seq(seq);
            if let Err(e) = ps.save(&state_dir) {
                tracing::warn!(target: "system", error = %e, "Failed to save agent state");
            }
        }
    });
}

fn resolve_secret(config: &AgentConfig) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if let Some(ref b64) = config.secret {
        return Ok(STANDARD.decode(b64)?);
    }
    if let Ok(val) = std::env::var("SENTINEL_AGENT_SECRET") {
        return Ok(val.into_bytes());
    }
    if let Ok(val) = std::env::var("SENTINEL_MASTER_KEY") {
        return Ok(val.into_bytes());
    }
    tracing::warn!(target: "auth", "No secret configured, HMAC signing will use an empty key");
    Ok(Vec::new())
}

fn spawn_plugin_scheduler(
    config: &AgentConfig,
    tx: mpsc::Sender<Vec<sentinel_common::proto::Metric>>,
) {
    if !config.plugins.enabled {
        tracing::info!(target: "plugin", "Plugin system disabled");
        return;
    }

    let mut scheduler = PluginScheduler::new(config.plugins.clone());
    scheduler.discover();

    if scheduler.loaded_count() > 0 {
        let _handle = scheduler.spawn(tx);
        tracing::info!(target: "plugin", "Plugin scheduler started");
    }
}
