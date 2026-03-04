use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{input, print_json, select, spinner, theme, OutputMode};

const NOTIFIER_TYPES: &[&str] = &["webhook", "slack", "discord", "smtp"];

#[derive(Args)]
pub struct CreateArgs {
    #[arg(long, help = "Notifier name")]
    pub name: Option<String>,

    #[arg(long, help = "Notifier type")]
    pub r#type: Option<String>,

    #[arg(long, help = "Target URL / webhook URL")]
    pub target: Option<String>,

    #[arg(long, help = "Optional secret")]
    pub secret: Option<String>,
}

pub async fn run(args: CreateArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let name = match args.name {
        Some(n) => n,
        None => input::text_required("Notifier name")?,
    };

    let ntype = match args.r#type {
        Some(t) => t,
        None if mode == OutputMode::Human => {
            let idx = select::select_option("Notifier type", NOTIFIER_TYPES).unwrap_or(0);
            NOTIFIER_TYPES[idx].to_string()
        }
        None => anyhow::bail!("--type is required in JSON mode"),
    };

    let target = match args.target {
        Some(t) => t,
        None => input::text_required("Target URL")?,
    };

    let config = build_config(&ntype, &target, args.secret.as_deref());

    let body = serde_json::json!({
        "name": name,
        "ntype": ntype,
        "config": config,
    });

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Creating notifier...")),
        OutputMode::Json => None,
    };

    let created = api.post_json("/v1/notifiers", &body).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Notifier created");
    }

    match mode {
        OutputMode::Json => print_json(&created)?,
        OutputMode::Human => {
            theme::print_kv("ID", created["id"].as_str().unwrap_or("-"));
            theme::print_kv("Name", created["name"].as_str().unwrap_or("-"));
        }
    }

    Ok(())
}

fn build_config(ntype: &str, target: &str, secret: Option<&str>) -> serde_json::Value {
    match ntype {
        "webhook" => {
            let mut cfg = serde_json::json!({ "url": target });
            if let Some(s) = secret {
                cfg["secret"] = serde_json::json!(s);
            }
            cfg
        }
        "slack" | "discord" => serde_json::json!({ "webhook_url": target }),
        "smtp" => serde_json::json!({ "host": target }),
        _ => serde_json::json!({ "url": target }),
    }
}
