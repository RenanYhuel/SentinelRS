use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct LinkArgs {
    #[arg(help = "Rule ID")]
    pub rule_id: String,

    #[arg(long, num_args = 1.., help = "Notifier IDs to attach")]
    pub notifier_ids: Vec<String>,
}

pub async fn run(args: LinkArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Linking notifiers to rule...")),
        OutputMode::Json => None,
    };

    let body = serde_json::json!({
        "notifier_ids": args.notifier_ids,
    });

    let updated = api
        .put_json(&format!("/v1/rules/{}", args.rule_id), &body)
        .await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Notifiers linked");
    }

    match mode {
        OutputMode::Json => print_json(&updated)?,
        OutputMode::Human => {
            theme::print_kv("Rule", updated["id"].as_str().unwrap_or("-"));
            let ids = updated["notifier_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            theme::print_kv("Notifiers", &ids);
        }
    }

    Ok(())
}
