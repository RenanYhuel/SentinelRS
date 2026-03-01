use anyhow::Result;
use clap::Subcommand;

use crate::output::{OutputMode, print_json, spinner, theme, select};

const NOTIFIER_TYPES: &[&str] = &["webhook", "slack", "discord", "smtp"];

#[derive(Subcommand)]
pub enum NotifiersCmd {
    Test(TestArgs),
}

#[derive(clap::Args)]
pub struct TestArgs {
    #[arg(long, help = "Notifier type: webhook, slack, discord, smtp")]
    r#type: Option<String>,

    #[arg(long, help = "Target URL or address")]
    target: Option<String>,

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
    let notifier_type = match args.r#type {
        Some(t) => t,
        None if mode == OutputMode::Human => {
            let idx = select::select_option("Select notifier type", NOTIFIER_TYPES)
                .ok_or_else(|| anyhow::anyhow!("no type selected"))?;
            NOTIFIER_TYPES[idx].to_string()
        }
        None => anyhow::bail!("--type is required in JSON mode"),
    };

    let target = match args.target {
        Some(t) => t,
        None => {
            let input = dialoguer::Input::<String>::with_theme(
                &dialoguer::theme::ColorfulTheme::default(),
            )
            .with_prompt("Target URL or address")
            .interact_text()?;
            input
        }
    };

    let base = server.unwrap_or_else(|| "http://localhost:8080".to_string());
    let url = format!("{base}/v1/notifiers/test");

    let mut body = serde_json::json!({
        "type": notifier_type,
        "target": target,
    });

    if let Some(s) = &args.secret {
        body["secret"] = serde_json::json!(s);
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create(&format!("Testing {notifier_type} notifier..."))),
        OutputMode::Json => None,
    };

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&body).send().await?;
    let status = resp.status();
    let result: serde_json::Value = resp.json().await.unwrap_or_default();

    if status.is_success() {
        if let Some(sp) = sp {
            spinner::finish_ok(&sp, &format!("Notifier test ({notifier_type}) succeeded"));
        }
        if mode == OutputMode::Json {
            print_json(&result)?;
        }
    } else {
        if let Some(sp) = sp {
            spinner::finish_err(&sp, &format!("Notifier test failed (HTTP {})", status.as_u16()));
        }
        match mode {
            OutputMode::Json => print_json(&serde_json::json!({
                "success": false,
                "status": status.as_u16(),
                "detail": result,
            }))?,
            OutputMode::Human => {
                theme::print_kv("Detail", &result.to_string());
            }
        }
    }

    Ok(())
}
