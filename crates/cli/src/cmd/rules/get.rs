use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct GetArgs {
    #[arg(help = "Rule ID")]
    pub id: String,
}

pub async fn run(args: GetArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching rule...")),
        OutputMode::Json => None,
    };

    let rule = api.get_json(&format!("/v1/rules/{}", args.id)).await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&rule)?,
        OutputMode::Human => {
            theme::print_header("Rule Details");
            for (k, v) in rule.as_object().into_iter().flatten() {
                theme::print_kv(k, &v.to_string());
            }
            println!();
        }
    }

    Ok(())
}
