use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{input, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct AddArgs {
    #[arg(help = "Agent name")]
    pub name: Option<String>,

    #[arg(long, help = "Comma-separated labels")]
    pub labels: Option<String>,

    #[arg(long, help = "Deploy as Docker container")]
    pub docker: bool,
}

pub async fn run(args: AddArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let name = match args.name {
        Some(n) => n,
        None => input::text_required("Agent name")?,
    };

    let labels: Vec<String> = match args.labels {
        Some(l) => l.split(',').map(|s| s.trim().to_string()).collect(),
        None => Vec::new(),
    };

    let body = serde_json::json!({
        "name": name,
        "labels": labels,
        "ttl_minutes": 10,
    });

    let sp = spinner::create("Generating agent token...");
    let result = api.post_json("/v1/agents/generate-install", &body).await?;
    spinner::finish_ok(&sp, "Token generated");

    let token = result["token"].as_str().unwrap_or("");

    if args.docker {
        deploy_docker_agent(&name, token, mode).await?;
    } else if mode == OutputMode::Human {
        theme::print_kv("Agent", &name);
        theme::print_dim("  Token passed internally — not displayed.");
        theme::print_dim("  Use --docker to auto-deploy as container.");
    }

    if mode == OutputMode::Json {
        print_json(&serde_json::json!({
            "name": name,
            "agent_id": result["agent_id"],
            "deployed": args.docker,
        }))?;
    }

    Ok(())
}

async fn deploy_docker_agent(name: &str, token: &str, mode: OutputMode) -> Result<()> {
    let container_name = format!("sentinel-{name}");

    let cfg = crate::store::load().unwrap_or_default();
    let grpc_url = &cfg.server.grpc_url;

    let sp = spinner::create(&format!("Deploying container '{container_name}'..."));

    let status = tokio::process::Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            &container_name,
            "-e",
            &format!("SENTINEL_TOKEN={token}"),
            "-e",
            &format!("SENTINEL_GRPC_URL={grpc_url}"),
            "--restart",
            "unless-stopped",
            "sentinelrs/agent:latest",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .await;

    match status {
        Ok(s) if s.success() => {
            spinner::finish_ok(&sp, &format!("Agent '{name}' deployed"));
            if mode == OutputMode::Human {
                wait_for_agent(name).await;
            }
        }
        _ => {
            spinner::finish_err(&sp, "Docker deployment failed");
            anyhow::bail!("failed to start container '{container_name}'");
        }
    }

    Ok(())
}

async fn wait_for_agent(name: &str) {
    let sp = spinner::create("Waiting for agent bootstrap...");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    spinner::finish_ok(&sp, &format!("Agent '{name}' is ONLINE"));
}
