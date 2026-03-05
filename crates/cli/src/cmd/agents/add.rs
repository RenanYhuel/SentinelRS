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
    let server_url = &cfg.server.grpc_url;
    let project = if cfg.docker.project_name.is_empty() {
        "sentinel".to_string()
    } else {
        cfg.docker.project_name.clone()
    };
    let network = format!("{project}_sentinel-net");

    cleanup_existing_container(&container_name).await;

    let sp = spinner::create(&format!("Deploying container '{container_name}'..."));

    let output = tokio::process::Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            &container_name,
            "-e",
            &format!("BOOTSTRAP_TOKEN={token}"),
            "-e",
            &format!("SERVER_URL={server_url}"),
            "-v",
            &format!("{container_name}-config:/etc/sentinel"),
            "--network",
            &network,
            "--restart",
            "unless-stopped",
            "sentinelrs/agent:latest",
        ])
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            spinner::finish_ok(&sp, &format!("Agent '{name}' deployed"));
            if mode == OutputMode::Human {
                wait_for_agent(name, &container_name).await;
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            spinner::finish_err(&sp, "Docker deployment failed");
            anyhow::bail!(
                "failed to start container '{container_name}': {}",
                stderr.trim()
            );
        }
        Err(e) => {
            spinner::finish_err(&sp, "Docker deployment failed");
            anyhow::bail!("failed to run docker: {e}");
        }
    }

    Ok(())
}

async fn cleanup_existing_container(container_name: &str) {
    let _ = tokio::process::Command::new("docker")
        .args(["rm", "-f", container_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
}

async fn wait_for_agent(name: &str, container_name: &str) {
    let sp = spinner::create("Waiting for agent bootstrap...");

    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let output = tokio::process::Command::new("docker")
            .args(["inspect", "-f", "{{.State.Running}}", container_name])
            .output()
            .await;

        match output {
            Ok(o) if String::from_utf8_lossy(&o.stdout).trim() == "true" => {
                spinner::finish_ok(&sp, &format!("Agent '{name}' is running"));
                return;
            }
            _ => continue,
        }
    }

    spinner::finish_err(&sp, &format!("Agent '{name}' did not start in time"));
}
