use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{print_json, spinner, OutputMode};

#[derive(Args)]
pub struct UpdateArgs {
    #[arg(help = "Rule ID")]
    pub id: String,

    #[arg(long, help = "JSON file path or inline JSON")]
    pub data: String,
}

pub async fn run(args: UpdateArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let body = if std::path::Path::new(&args.data).exists() {
        let content = std::fs::read_to_string(&args.data)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::from_str(&args.data)?
    };

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Updating rule...")),
        OutputMode::Json => None,
    };

    let updated = api
        .put_json(&format!("/v1/rules/{}", args.id), &body)
        .await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, &format!("Rule {} updated", args.id));
    }

    if mode == OutputMode::Json {
        print_json(&updated)?;
    }

    Ok(())
}
