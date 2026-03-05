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
    let server_url = resolve_docker_grpc_url(&cfg);
    let network = resolve_compose_network(&cfg);

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
            &format!("SERVER_URL={}", server_url),
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

    for _ in 0..15 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let output = tokio::process::Command::new("docker")
            .args(["logs", "--tail", "5", container_name])
            .output()
            .await;

        if let Ok(o) = output {
            let logs = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            let combined = format!("{logs}{stderr}");

            if combined.contains("Bootstrap complete")
                || combined.contains("Agent provisioned")
                || combined.contains("Configuration loaded")
            {
                spinner::finish_ok(&sp, &format!("Agent '{name}' bootstrapped"));
                return;
            }

            if combined.contains("Bootstrap failed after") {
                spinner::finish_err(
                    &sp,
                    &format!(
                        "Agent '{name}' bootstrap failed — check: docker logs {container_name}"
                    ),
                );
                return;
            }
        }
    }

    spinner::finish_err(
        &sp,
        &format!("Agent '{name}' still bootstrapping — check: docker logs {container_name}"),
    );
}

fn resolve_docker_grpc_url(cfg: &crate::store::config::CliConfig) -> String {
    let port = cfg
        .server
        .grpc_url
        .rsplit(':')
        .next()
        .and_then(|p| p.trim_end_matches('/').parse::<u16>().ok())
        .unwrap_or(50051);
    format!("http://sentinel-server:{port}")
}

fn resolve_compose_network(cfg: &crate::store::config::CliConfig) -> String {
    if !cfg.docker.compose_file.is_empty() {
        if let Some(parent) = std::path::Path::new(&cfg.docker.compose_file).parent() {
            let dir_name = parent
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("deploy");
            if !dir_name.is_empty() && dir_name != "." {
                return format!("{dir_name}_sentinel-net");
            }
        }
    }
    "deploy_sentinel-net".to_string()
}
