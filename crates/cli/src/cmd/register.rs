use anyhow::{Context, Result};
use clap::Args;
use sentinel_common::redact::mask_token;
use tonic::transport::Channel;

use crate::output::{input, print_info, print_json, spinner, theme, OutputMode};
use sentinel_common::proto::agent_service_client::AgentServiceClient;
use sentinel_common::proto::RegisterRequest;

#[derive(Args)]
pub struct RegisterArgs {
    #[arg(long)]
    hw_id: Option<String>,

    #[arg(long, default_value = env!("CARGO_PKG_VERSION"))]
    agent_version: String,

    #[arg(long)]
    save: bool,

    #[arg(long, help = "Show secret in clear text")]
    reveal: bool,
}

pub async fn run(args: RegisterArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let hw_id = match args.hw_id {
        Some(id) => id,
        None => input::text_required("Hardware ID")?,
    };

    let url = resolve_grpc_endpoint(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Connecting to server...")),
        OutputMode::Json => None,
    };

    let channel = Channel::from_shared(url)
        .context("invalid endpoint")?
        .connect()
        .await
        .context("failed to connect to server")?;

    if let Some(sp) = &sp {
        sp.set_message("Registering agent...");
    }

    let mut grpc = AgentServiceClient::new(channel);

    let response = grpc
        .register(RegisterRequest {
            hw_id: hw_id.clone(),
            agent_version: args.agent_version.clone(),
        })
        .await
        .context("register RPC failed")?
        .into_inner();

    if args.save {
        save_credentials(&response.agent_id, &response.secret)?;
    }

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Agent registered successfully");
    }

    let display_secret = if args.reveal {
        response.secret.clone()
    } else {
        mask_token(&response.secret)
    };

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "agent_id": response.agent_id,
            "secret": display_secret,
        }))?,
        OutputMode::Human => {
            theme::print_section("Credentials");
            theme::print_kv("Agent ID", &response.agent_id);
            theme::print_kv("Secret", &display_secret);
            if args.save {
                println!();
                print_info("Saved", "credentials stored locally");
            }
        }
    }

    Ok(())
}

fn resolve_grpc_endpoint(server: Option<&str>) -> Result<String> {
    if let Some(s) = server {
        return Ok(s.to_string());
    }
    if let Ok(cfg) = crate::store::load() {
        let mut url = cfg.server.grpc_url.clone();
        if !url.starts_with("http") {
            url = format!("http://{url}");
        }
        return Ok(url);
    }
    let url = input::text("gRPC endpoint", "http://localhost:50051")?;
    Ok(url)
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
