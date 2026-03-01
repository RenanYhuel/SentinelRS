use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

use crate::api::{self, AgentState};
use crate::batch::BatchComposer;
use crate::buffer::Wal;
use crate::collector::SystemCollector;
use crate::config::AgentConfig;
use crate::exporter::{GrpcClient, RetryPolicy, SendLoop};
use crate::scheduler::ScheduledTask;

pub async fn run(config: AgentConfig) -> Result<(), Box<dyn std::error::Error>> {
    let agent_id = config
        .agent_id
        .clone()
        .unwrap_or_else(|| sysinfo::System::host_name().unwrap_or_else(|| "unknown".into()));

    let secret = resolve_secret(&config)?;

    tracing::info!(
        agent_id = %agent_id,
        server = %config.server,
        interval_s = config.collect.interval_seconds,
        "agent configured"
    );

    let segment_bytes = config.buffer.segment_size_mb * 1024 * 1024;
    let wal = Wal::open(Path::new(&config.buffer.wal_dir), true, segment_bytes)?;
    let wal = Arc::new(Mutex::new(wal));

    let state = AgentState::new();

    let (metrics_tx, metrics_rx) = mpsc::channel(256);

    spawn_collector(config.collect.interval_seconds, metrics_tx);
    spawn_batcher(agent_id.clone(), wal.clone(), metrics_rx);
    spawn_sender(
        config.server.clone(),
        agent_id,
        secret,
        wal.clone(),
        state.clone(),
    );
    spawn_api(config.api_port, state).await;

    tracing::info!("agent running");
    crate::shutdown::wait_for_shutdown().await;

    tracing::info!("shutting down");
    let w = wal.lock().await;
    let _ = w.save_meta();

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
) {
    tokio::spawn(async move {
        let mut composer = BatchComposer::new(agent_id, 0);
        while let Some(metrics) = rx.recv().await {
            let batch = composer.compose(metrics);
            let encoded = BatchComposer::encode_batch(&batch);
            let mut w = wal.lock().await;
            if let Err(e) = w.append(encoded) {
                tracing::error!(error = %e, "WAL write failed");
            }
        }
    });
}

fn spawn_sender(
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
            match GrpcClient::connect(&server, agent_id.clone(), &secret, "default".into()).await {
                Ok(mut client) => {
                    tracing::info!("connected to server");
                    state.set_ready(true);

                    loop {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        let mut w = wal.lock().await;
                        match send_loop.send_pending(&mut w, &mut client).await {
                            Ok(n) if n > 0 => {
                                for _ in 0..n {
                                    state.increment_batches_sent();
                                }
                                tracing::debug!(sent = n, "batches sent");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(error = %e, "send failed, will reconnect");
                                state.increment_batches_failed();
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "server unreachable, retrying in 10s");
                    state.set_ready(false);
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
}

async fn spawn_api(port: u16, state: AgentState) {
    let addr = format!("0.0.0.0:{port}");
    tokio::spawn(async move {
        match TcpListener::bind(&addr).await {
            Ok(listener) => {
                tracing::info!(addr = %addr, "HTTP API listening");
                if let Err(e) = api::serve(listener, state).await {
                    tracing::error!(error = %e, "HTTP API error");
                }
            }
            Err(e) => tracing::error!(error = %e, addr = %addr, "failed to bind HTTP API"),
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
    tracing::warn!("no secret configured, HMAC signing will use an empty key");
    Ok(Vec::new())
}
