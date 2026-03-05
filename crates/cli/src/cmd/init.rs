use anyhow::Result;

use crate::output::{banner, input, print_success, select, spinner, theme, OutputMode};
use crate::store::{self, CliConfig};

const DEPLOY_TYPES: &[&str] = &[
    "Full server (Server + DB + NATS + Worker)",
    "Additional worker",
    "Agent standalone",
];

pub async fn execute(mode: OutputMode) -> Result<()> {
    if mode == OutputMode::Human {
        banner::print_banner();
        theme::print_header("Setup Wizard");
    }

    let deploy_idx = if mode == OutputMode::Human {
        select::select_option("Deployment type", DEPLOY_TYPES).unwrap_or(0)
    } else {
        0
    };

    let server_url = input::text("Server REST URL", "http://localhost:8080")?;
    let grpc_url = input::text("Server gRPC URL", "http://localhost:50051")?;

    let sp = spinner::create("Testing connection...");
    let client = crate::client::ApiClient::new(&server_url);
    let reachable = client.health_ok().await;
    if reachable {
        spinner::finish_ok(&sp, "Server reachable");
    } else {
        spinner::finish_err(&sp, "Server unreachable (config saved anyway)");
    }

    let docker_available = check_docker().await;
    if mode == OutputMode::Human {
        if docker_available {
            theme::print_dim("  Docker detected");
        } else {
            theme::print_dim("  Docker not found — container features disabled");
        }
    }

    let output_options = &["human", "json"];
    let output_idx = select::select_option("Default output format", output_options).unwrap_or(0);

    let mut cfg = CliConfig::default();
    cfg.server.url = server_url.clone();
    cfg.server.grpc_url = grpc_url;
    cfg.defaults.output_format = output_options[output_idx].to_string();

    if docker_available && deploy_idx == 0 {
        let compose = input::text_optional("Docker compose file path")?;
        if let Some(path) = compose {
            cfg.docker.compose_file = path;
        }
    }

    if reachable {
        if let Ok(secret) = input::password("JWT secret (from server config)") {
            let token = fetch_token(&server_url, &secret).await;
            match token {
                Ok(t) => {
                    cfg.auth.jwt_token = t;
                    if mode == OutputMode::Human {
                        theme::print_dim("  Authenticated successfully");
                    }
                }
                Err(e) => {
                    if mode == OutputMode::Human {
                        theme::print_dim(&format!("  Authentication failed: {e}"));
                    }
                }
            }
        }
    }

    store::save(&cfg)?;

    if mode == OutputMode::Human {
        println!();
        print_success(&format!(
            "Config saved to {}",
            store::config_path().display()
        ));
        println!();
        theme::print_section("Saved Configuration");
        theme::print_kv("Server URL", cfg.server_url());
        theme::print_kv("gRPC URL", &cfg.server.grpc_url);
        theme::print_kv("Output", cfg.output());
        if !cfg.docker.compose_file.is_empty() {
            theme::print_kv("Compose", &cfg.docker.compose_file);
        }
        println!();
        theme::print_dim("  Next: run 'sentinel agent add <name>' to connect your first agent.");
        println!();
    }

    Ok(())
}

async fn check_docker() -> bool {
    tokio::process::Command::new("docker")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn fetch_token(server_url: &str, secret: &str) -> Result<String> {
    let url = format!("{}/v1/auth/token", server_url.trim_end_matches('/'));
    let body = serde_json::json!({ "secret": secret });
    let resp = reqwest::Client::new().post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("invalid secret or server rejected request");
    }
    let json: serde_json::Value = resp.json().await?;
    json["token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("missing token in response"))
}
