use tokio_util::sync::CancellationToken;

pub fn spawn_signal_handler(cancel: CancellationToken) {
    tokio::spawn(async move {
        wait_for_signal().await;
        tracing::info!(target: "system", "Shutdown signal received");
        cancel.cancel();
    });
}

#[cfg(unix)]
async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_signal() {
    tokio::signal::ctrl_c().await.expect("Ctrl-C handler");
}
