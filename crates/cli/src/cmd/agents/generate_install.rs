use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{input, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct GenerateArgs {
    #[arg(long, help = "Agent display name")]
    pub name: Option<String>,

    #[arg(long, help = "Comma-separated labels")]
    pub labels: Option<String>,

    #[arg(long, default_value = "30", help = "Token TTL in minutes")]
    pub ttl: i64,
}

pub async fn run(args: GenerateArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let name = match args.name {
        Some(n) => n,
        None => input::text_required("Agent name")?,
    };

    let labels: Vec<String> = match args.labels {
        Some(l) => l.split(',').map(|s| s.trim().to_string()).collect(),
        None => {
            let raw = input::text_optional("Labels (comma-separated)")?;
            raw.map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default()
        }
    };

    let body = serde_json::json!({
        "name": name,
        "labels": labels,
        "ttl_minutes": args.ttl,
    });

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Generating install command...")),
        OutputMode::Json => None,
    };

    let result = api.post_json("/v1/agents/generate-install", &body).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Install command generated");
    }

    match mode {
        OutputMode::Json => print_json(&result)?,
        OutputMode::Human => {
            theme::print_header("Install Command");
            if let Some(cmd) = result["install_command"].as_str() {
                println!("  {cmd}");
            }
            println!();
            theme::print_section("Details");
            theme::print_kv("Token", result["token"].as_str().unwrap_or("-"));
            theme::print_kv("Expires", result["expires_at"].as_str().unwrap_or("-"));
            println!();
        }
    }

    Ok(())
}
