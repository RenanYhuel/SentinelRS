use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{input, print_json, select, spinner, theme, OutputMode};

const NOTIFIER_TYPES: &[&str] = &[
    "webhook",
    "slack",
    "discord",
    "smtp",
    "telegram",
    "pagerduty",
    "teams",
    "opsgenie",
    "gotify",
    "ntfy",
];

#[derive(Args)]
pub struct TestArgs {
    #[arg(long, help = "Notifier type")]
    pub r#type: Option<String>,

    #[arg(long, help = "Target URL or address")]
    pub target: Option<String>,

    #[arg(long, help = "Optional secret/token")]
    pub secret: Option<String>,
}

pub async fn run(args: TestArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let notifier_type = match args.r#type {
        Some(t) => t,
        None if mode == OutputMode::Human => {
            let idx = select::select_option("Select notifier type", NOTIFIER_TYPES)
                .ok_or_else(|| anyhow::anyhow!("cancelled"))?;
            NOTIFIER_TYPES[idx].to_string()
        }
        None => anyhow::bail!("--type is required in JSON mode"),
    };

    let target = match args.target {
        Some(t) => t,
        None => input::text_required("Target URL or address")?,
    };

    let mut body = serde_json::json!({
        "type": notifier_type,
        "target": target,
    });

    if let Some(s) = &args.secret {
        body["secret"] = serde_json::json!(s);
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create(&format!(
            "Testing {notifier_type} notifier..."
        ))),
        OutputMode::Json => None,
    };

    let result = api.post_json("/v1/notifiers/test", &body).await?;

    let success = result["success"].as_bool().unwrap_or(false);

    if let Some(sp) = sp {
        if success {
            spinner::finish_ok(&sp, &format!("{notifier_type} test succeeded"));
        } else {
            spinner::finish_err(&sp, &format!("{notifier_type} test failed"));
        }
    }

    match mode {
        OutputMode::Json => print_json(&result)?,
        OutputMode::Human => {
            if let Some(msg) = result["message"].as_str() {
                theme::print_kv("Detail", msg);
            }
        }
    }

    Ok(())
}
