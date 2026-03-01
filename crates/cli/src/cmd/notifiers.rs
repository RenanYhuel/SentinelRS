use anyhow::Result;
use clap::Subcommand;

use crate::output::{OutputMode, print_json, print_success, print_error};

#[derive(Subcommand)]
pub enum NotifiersCmd {
    Test(TestArgs),
}

#[derive(clap::Args)]
pub struct TestArgs {
    #[arg(long, help = "Notifier type: webhook, slack, discord, smtp")]
    r#type: String,

    #[arg(long, help = "Target URL or address")]
    target: String,

    #[arg(long, help = "Optional secret/token")]
    secret: Option<String>,
}

pub async fn execute(
    cmd: NotifiersCmd,
    mode: OutputMode,
    server: Option<String>,
) -> Result<()> {
    match cmd {
        NotifiersCmd::Test(args) => test(args, mode, server).await,
    }
}

async fn test(
    args: TestArgs,
    mode: OutputMode,
    server: Option<String>,
) -> Result<()> {
    let base = server.unwrap_or_else(|| "http://localhost:8080".to_string());
    let url = format!("{base}/v1/notifiers/test");

    let mut body = serde_json::json!({
        "type": args.r#type,
        "target": args.target,
    });

    if let Some(s) = &args.secret {
        body["secret"] = serde_json::json!(s);
    }

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&body).send().await?;

    let status = resp.status();
    let result: serde_json::Value = resp.json().await.unwrap_or_default();

    if status.is_success() {
        match mode {
            OutputMode::Json => print_json(&result)?,
            OutputMode::Human => print_success(&format!("Notifier test ({}) succeeded", args.r#type)),
        }
    } else {
        match mode {
            OutputMode::Json => print_json(&serde_json::json!({
                "success": false,
                "status": status.as_u16(),
                "detail": result,
            }))?,
            OutputMode::Human => print_error(&format!(
                "Notifier test failed (HTTP {}): {}",
                status.as_u16(),
                result
            )),
        }
    }

    Ok(())
}
