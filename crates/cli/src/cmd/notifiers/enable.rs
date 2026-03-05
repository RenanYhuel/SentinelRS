use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct EnableArgs {
    #[arg(help = "Notifier ID to toggle")]
    pub id: String,
}

pub async fn run(args: EnableArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Toggling notifier...")),
        OutputMode::Json => None,
    };

    let url = format!("/v1/notifiers/{}/toggle", args.id);
    let result = api.post_empty(&url).await?;

    if let Some(sp) = sp {
        let enabled = result["enabled"].as_bool().unwrap_or(false);
        let status = if enabled { "enabled" } else { "disabled" };
        spinner::finish_ok(&sp, &format!("Notifier {status}"));
    }

    match mode {
        OutputMode::Json => print_json(&result)?,
        OutputMode::Human => {
            theme::print_kv("ID", result["id"].as_str().unwrap_or("-"));
            theme::print_kv("Name", result["name"].as_str().unwrap_or("-"));
            theme::print_kv(
                "Enabled",
                &result["enabled"].as_bool().unwrap_or(false).to_string(),
            );
        }
    }

    Ok(())
}
