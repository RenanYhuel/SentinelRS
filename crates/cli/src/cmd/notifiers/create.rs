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
pub struct CreateArgs {
    #[arg(long, help = "Notifier name")]
    pub name: Option<String>,

    #[arg(long, help = "Notifier type")]
    pub r#type: Option<String>,

    #[arg(long, help = "Target URL / webhook URL")]
    pub target: Option<String>,

    #[arg(long, help = "Optional secret / token")]
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

    let config = if mode == OutputMode::Human && args.target.is_none() {
        build_config_interactive(&ntype, args.secret.as_deref())?
    } else {
        let target = match args.target {
            Some(t) => t,
            None => input::text_required("Target URL")?,
        };
        build_config(&ntype, &target, args.secret.as_deref())
    };

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

fn build_config_interactive(
    ntype: &str,
    secret: Option<&str>,
) -> Result<serde_json::Value> {
    match ntype {
        "webhook" => {
            let url = input::text_required("Webhook URL")?;
            let mut cfg = serde_json::json!({ "url": url });
            if let Some(s) = secret {
                cfg["secret"] = serde_json::json!(s);
            } else {
                let s = input::text_optional("HMAC secret (optional)")?;
                if let Some(s) = s {
                    cfg["secret"] = serde_json::json!(s);
                }
            }
            Ok(cfg)
        }
        "slack" => {
            let url = input::text_required("Slack webhook URL")?;
            Ok(serde_json::json!({ "webhook_url": url }))
        }
        "discord" => {
            let url = input::text_required("Discord webhook URL")?;
            Ok(serde_json::json!({ "webhook_url": url }))
        }
        "smtp" => {
            let host = input::text_required("SMTP host")?;
            let port = input::text_optional("SMTP port (default: 587)")?
                .unwrap_or_else(|| "587".to_string());
            let username = input::text_optional("Username")?;
            let password = input::text_optional("Password")?;
            let from = input::text_required("From address")?;
            let to = input::text_required("To address")?;
            let mut cfg = serde_json::json!({
                "host": host,
                "port": port.parse::<u16>().unwrap_or(587),
                "from": from,
                "to": to,
            });
            if let Some(u) = username {
                cfg["username"] = serde_json::json!(u);
            }
            if let Some(p) = password {
                cfg["password"] = serde_json::json!(p);
            }
            Ok(cfg)
        }
        "telegram" => {
            let bot_token = input::text_required("Bot token")?;
            let chat_id = input::text_required("Chat ID")?;
            Ok(serde_json::json!({
                "bot_token": bot_token,
                "chat_id": chat_id,
            }))
        }
        "pagerduty" => {
            let routing_key = input::text_required("Routing key (integration key)")?;
            Ok(serde_json::json!({ "routing_key": routing_key }))
        }
        "teams" => {
            let url = input::text_required("Teams webhook URL")?;
            Ok(serde_json::json!({ "webhook_url": url }))
        }
        "opsgenie" => {
            let api_key = input::text_required("OpsGenie API key")?;
            Ok(serde_json::json!({ "api_key": api_key }))
        }
        "gotify" => {
            let server_url = input::text_required("Gotify server URL")?;
            let token = input::text_required("Application token")?;
            Ok(serde_json::json!({
                "server_url": server_url,
                "token": token,
            }))
        }
        "ntfy" => {
            let server_url = input::text_optional("Ntfy server URL (default: https://ntfy.sh)")?
                .unwrap_or_else(|| "https://ntfy.sh".to_string());
            let topic = input::text_required("Topic")?;
            let token = input::text_optional("Access token (optional)")?;
            let mut cfg = serde_json::json!({
                "server_url": server_url,
                "topic": topic,
            });
            if let Some(t) = token {
                cfg["token"] = serde_json::json!(t);
            }
            Ok(cfg)
        }
        _ => {
            let url = input::text_required("Target URL")?;
            Ok(serde_json::json!({ "url": url }))
        }
    }
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
        "slack" | "discord" | "teams" => serde_json::json!({ "webhook_url": target }),
        "smtp" => serde_json::json!({ "host": target }),
        "telegram" => serde_json::json!({ "bot_token": target }),
        "pagerduty" => serde_json::json!({ "routing_key": target }),
        "opsgenie" => serde_json::json!({ "api_key": target }),
        "gotify" => serde_json::json!({ "server_url": target }),
        "ntfy" => serde_json::json!({ "server_url": target, "topic": "sentinel" }),
        _ => serde_json::json!({ "url": target }),
    }
}

