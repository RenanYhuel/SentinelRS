use anyhow::{Context, Result};
use clap::Args;
use tonic::transport::Channel;

use sentinel_common::proto::agent_service_client::AgentServiceClient;
use sentinel_common::proto::RegisterRequest;
use crate::output::{OutputMode, print_json, print_success, print_info};

#[derive(Args)]
pub struct RegisterArgs {
    #[arg(long, help = "Hardware ID for this agent")]
    hw_id: String,

    #[arg(long, default_value = env!("CARGO_PKG_VERSION"), help = "Agent version")]
    agent_version: String,

    #[arg(long, help = "Save credentials to config directory")]
    save: bool,
}

#[derive(serde::Serialize)]
struct RegisterOutput {
    agent_id: String,
    secret: String,
}

pub async fn execute(
    args: RegisterArgs,
    mode: OutputMode,
    server: Option<String>,
    config_path: Option<String>,
) -> Result<()> {
    let endpoint = super::helpers::resolve_server(
        server.as_deref(),
        config_path.as_deref(),
    )?;

    let channel = Channel::from_shared(endpoint.clone())
        .context("invalid endpoint")?
        .connect()
        .await
        .context("failed to connect to server")?;

    let mut client = AgentServiceClient::new(channel);

    let response = client
        .register(RegisterRequest {
            hw_id: args.hw_id.clone(),
            agent_version: args.agent_version.clone(),
        })
        .await
        .context("register RPC failed")?
        .into_inner();

    let out = RegisterOutput {
        agent_id: response.agent_id.clone(),
        secret: response.secret.clone(),
    };

    if args.save {
        save_credentials(&response.agent_id, &response.secret)?;
    }

    match mode {
        OutputMode::Json => print_json(&out)?,
        OutputMode::Human => {
            print_success("Agent registered successfully");
            print_info("Agent ID", &out.agent_id);
            print_info("Secret", &out.secret);
            if args.save {
                print_info("Saved", "credentials stored locally");
            }
        }
    }

    Ok(())
}

fn save_credentials(agent_id: &str, secret: &str) -> Result<()> {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("sentinel");

    std::fs::create_dir_all(&dir)?;

    let creds = serde_json::json!({
        "agent_id": agent_id,
        "secret": secret
    });

    let path = dir.join("credentials.json");
    std::fs::write(&path, serde_json::to_string_pretty(&creds)?)?;

    Ok(())
}
