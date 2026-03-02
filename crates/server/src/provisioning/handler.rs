use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use sentinel_common::proto::{BootstrapResponse, BootstrapStatus};

use crate::persistence::AgentRepo;
use crate::store::{AgentRecord, AgentStore};

use super::config_builder::build_agent_config_yaml;
use super::store::TokenStore;
use super::validator::{validate_and_consume, ValidationResult};

pub struct BootstrapOutcome {
    pub response: BootstrapResponse,
}

pub async fn handle_bootstrap(
    token_store: &TokenStore,
    agent_store: &AgentStore,
    agent_repo: Option<&AgentRepo>,
    bootstrap_token: &str,
    hw_id: &str,
    agent_version: &str,
    server_url: &str,
) -> BootstrapOutcome {
    let entry = match validate_and_consume(token_store, bootstrap_token) {
        ValidationResult::Valid(e) => e,
        ValidationResult::NotFound => {
            return reject(BootstrapStatus::BootstrapInvalidToken, "unknown token");
        }
        ValidationResult::Expired => {
            return reject(BootstrapStatus::BootstrapExpiredToken, "token expired");
        }
        ValidationResult::AlreadyConsumed => {
            return reject(BootstrapStatus::BootstrapInvalidToken, "token already used");
        }
    };

    let agent_id = if entry.agent_name.is_empty() {
        format!("agent-{}", uuid::Uuid::new_v4())
    } else {
        entry.agent_name.clone()
    };

    let secret = sentinel_common::crypto::generate_secret();
    let key_id = format!("key-{}", uuid::Uuid::new_v4());

    let now_ms = current_time_ms();

    let record = AgentRecord {
        agent_id: agent_id.clone(),
        hw_id: hw_id.into(),
        secret: secret.clone(),
        key_id: key_id.clone(),
        agent_version: agent_version.into(),
        registered_at_ms: now_ms,
        deprecated_keys: Vec::new(),
        last_seen: None,
    };

    agent_store.insert(record.clone());

    if let Some(repo) = agent_repo {
        if let Err(e) = repo.upsert(&record).await {
            tracing::error!(error = %e, agent_id = %agent_id, "failed to persist bootstrapped agent");
        }
    }

    let secret_b64 = STANDARD.encode(&secret);
    let config_yaml = build_agent_config_yaml(&agent_id, &secret_b64, server_url);

    tracing::info!(
        agent_id = %agent_id,
        hw_id = %hw_id,
        token_name = %entry.agent_name,
        "agent bootstrapped via zero-touch provisioning"
    );

    BootstrapOutcome {
        response: BootstrapResponse {
            status: BootstrapStatus::BootstrapOk.into(),
            agent_id,
            secret: secret_b64,
            key_id,
            config_yaml: config_yaml.into_bytes(),
            message: "provisioned".into(),
        },
    }
}

fn reject(status: BootstrapStatus, message: &str) -> BootstrapOutcome {
    BootstrapOutcome {
        response: BootstrapResponse {
            status: status.into(),
            agent_id: String::new(),
            secret: String::new(),
            key_id: String::new(),
            config_yaml: Vec::new(),
            message: message.into(),
        },
    }
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
