use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{input, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct UpdateArgs {
    #[arg(help = "Notifier ID to update")]
    pub id: String,

    #[arg(long, help = "New name")]
    pub name: Option<String>,

    #[arg(long, help = "Enable notifier")]
    pub enable: bool,

    #[arg(long, help = "Disable notifier")]
    pub disable: bool,
}

pub async fn run(args: UpdateArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let name = match args.name {
        Some(n) => Some(n),
        None if mode == OutputMode::Human => input::text_optional("New name (press Enter to keep)")?,
        None => None,
    };

    let enabled = if args.enable {
        Some(true)
    } else if args.disable {
        Some(false)
    } else {
        None
    };

    let mut body = serde_json::Map::new();
    if let Some(n) = name {
        body.insert("name".into(), serde_json::json!(n));
    }
    if let Some(e) = enabled {
        body.insert("enabled".into(), serde_json::json!(e));
    }

    if body.is_empty() {
        anyhow::bail!("nothing to update");
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Updating notifier...")),
        OutputMode::Json => None,
    };

    let url = format!("/v1/notifiers/{}", args.id);
    let result = api.put_json(&url, &serde_json::Value::Object(body)).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Notifier updated");
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
