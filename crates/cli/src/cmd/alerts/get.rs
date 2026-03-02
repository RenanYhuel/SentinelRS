use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct GetArgs {
    #[arg(help = "Alert ID")]
    pub id: String,
}

pub async fn run(args: GetArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching alert...")),
        OutputMode::Json => None,
    };

    let alert = api.get_json(&format!("/v1/alerts/{}", args.id)).await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&alert)?,
        OutputMode::Human => {
            theme::print_header("Alert Details");
            for (k, v) in alert.as_object().into_iter().flatten() {
                let display = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                theme::print_kv(k, &display);
            }
            println!();
        }
    }

    Ok(())
}
