mod cleanup;
mod config_writer;
mod detector;
mod negotiator;

use std::path::{Path, PathBuf};

pub use detector::{bootstrap_token_from_env, needs_bootstrap, server_url_from_env};
pub use negotiator::NegotiateError;

const DEFAULT_CONFIG_DIR: &str = "/etc/sentinel";
const CONFIG_FILE_NAME: &str = "config.yml";
const MAX_BOOTSTRAP_RETRIES: u32 = 5;
const BASE_BACKOFF_SECS: u64 = 5;

pub async fn run_if_needed(
    config_path: &Path,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    if !needs_bootstrap(config_path) {
        return Ok(None);
    }

    tracing::info!(target: "boot", "No config file found, entering bootstrap mode");

    let token = match bootstrap_token_from_env() {
        Some(t) => t,
        None => {
            return Err("BOOTSTRAP_TOKEN env var required for zero-touch provisioning".into());
        }
    };

    let server_url = match server_url_from_env() {
        Some(u) => u,
        None => {
            return Err("SERVER_URL env var required for zero-touch provisioning".into());
        }
    };

    let hw_id = sysinfo::System::host_name().unwrap_or_else(|| "unknown-hw".into());

    let mut last_err = None;
    for attempt in 1..=MAX_BOOTSTRAP_RETRIES {
        tracing::info!(
            target: "boot",
            server = %server_url,
            hw_id = %hw_id,
            attempt,
            max = MAX_BOOTSTRAP_RETRIES,
            "Negotiating bootstrap"
        );

        match negotiator::negotiate(&server_url, &token, &hw_id).await {
            Ok(result) => {
                tracing::info!(
                    target: "boot",
                    agent_id = %result.agent_id,
                    "Bootstrap successful, writing config"
                );

                let config_dir = config_path
                    .parent()
                    .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_DIR));

                config_writer::write_config(config_dir, &result.config_yaml)?;
                cleanup::scrub_token_from_env();

                let written_path = config_dir.join(CONFIG_FILE_NAME);
                tracing::info!(
                    target: "boot",
                    path = %written_path.display(),
                    "Agent provisioned, reloading config"
                );

                return Ok(Some(written_path));
            }
            Err(e) => {
                let delay = BASE_BACKOFF_SECS * 2u64.pow(attempt - 1);
                tracing::warn!(
                    target: "boot",
                    error = %e,
                    attempt,
                    retry_in_secs = delay,
                    "Bootstrap attempt failed"
                );
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }
    }

    Err(format!(
        "Bootstrap failed after {MAX_BOOTSTRAP_RETRIES} attempts: {}",
        last_err.map(|e| e.to_string()).unwrap_or_default()
    )
    .into())
}
