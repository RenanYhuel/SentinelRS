use anyhow::Result;
use clap::Subcommand;

use crate::output::{OutputMode, print_json, print_success, build_table, spinner, theme, confirm};
use super::helpers;

#[derive(Subcommand)]
pub enum RulesCmd {
    List,
    Get(GetArgs),
    Create(CreateArgs),
    Update(UpdateArgs),
    Delete(DeleteArgs),
}

#[derive(clap::Args)]
pub struct GetArgs {
    #[arg(help = "Rule ID")]
    id: String,
}

#[derive(clap::Args)]
pub struct CreateArgs {
    #[arg(long, help = "JSON file path or inline JSON")]
    data: String,
}

#[derive(clap::Args)]
pub struct UpdateArgs {
    #[arg(help = "Rule ID")]
    id: String,
    #[arg(long, help = "JSON file path or inline JSON")]
    data: String,
}

#[derive(clap::Args)]
pub struct DeleteArgs {
    #[arg(help = "Rule ID")]
    id: String,
    #[arg(long, help = "Skip confirmation prompt")]
    yes: bool,
}

pub async fn execute(
    cmd: RulesCmd,
    mode: OutputMode,
    server: Option<String>,
    config_path: Option<String>,
) -> Result<()> {
    let base = helpers::resolve_rest_url(server.as_deref(), config_path.as_deref())?;

    match cmd {
        RulesCmd::List => list(&base, mode).await,
        RulesCmd::Get(args) => get(&base, args, mode).await,
        RulesCmd::Create(args) => create(&base, args, mode).await,
        RulesCmd::Update(args) => update(&base, args, mode).await,
        RulesCmd::Delete(args) => delete(&base, args, mode).await,
    }
}

async fn list(base: &str, mode: OutputMode) -> Result<()> {
    let url = format!("{base}/v1/rules");

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching rules...")),
        OutputMode::Json => None,
    };

    let resp = reqwest::get(&url).await?.error_for_status()?;
    let rules: Vec<serde_json::Value> = resp.json().await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&rules)?,
        OutputMode::Human => {
            if rules.is_empty() {
                print_success("No alert rules defined");
                return Ok(());
            }
            theme::print_header("Alert Rules");
            let mut table = build_table(&["ID", "Name", "Metric", "Condition", "Threshold"]);
            for r in &rules {
                table.add_row(vec![
                    r["id"].as_str().unwrap_or("-"),
                    r["name"].as_str().unwrap_or("-"),
                    r["metric_name"].as_str().unwrap_or("-"),
                    r["condition"].as_str().unwrap_or("-"),
                    &r["threshold"].to_string(),
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}

async fn get(base: &str, args: GetArgs, mode: OutputMode) -> Result<()> {
    let url = format!("{base}/v1/rules/{}", args.id);

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching rule...")),
        OutputMode::Json => None,
    };

    let resp = reqwest::get(&url).await?.error_for_status()?;
    let rule: serde_json::Value = resp.json().await?;

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

fn parse_json_data(data: &str) -> Result<serde_json::Value> {
    if std::path::Path::new(data).exists() {
        let content = std::fs::read_to_string(data)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(serde_json::from_str(data)?)
    }
}

async fn create(base: &str, args: CreateArgs, mode: OutputMode) -> Result<()> {
    let body = parse_json_data(&args.data)?;
    let url = format!("{base}/v1/rules");

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Creating rule...")),
        OutputMode::Json => None,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    let created: serde_json::Value = resp.json().await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Rule created");
    }

    match mode {
        OutputMode::Json => print_json(&created)?,
        OutputMode::Human => {
            theme::print_kv("ID", created["id"].as_str().unwrap_or("-"));
        }
    }

    Ok(())
}

async fn update(base: &str, args: UpdateArgs, mode: OutputMode) -> Result<()> {
    let body = parse_json_data(&args.data)?;
    let url = format!("{base}/v1/rules/{}", args.id);

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Updating rule...")),
        OutputMode::Json => None,
    };

    let client = reqwest::Client::new();
    let resp = client
        .put(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    let updated: serde_json::Value = resp.json().await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, &format!("Rule {} updated", args.id));
    }

    if mode == OutputMode::Json {
        print_json(&updated)?;
    }

    Ok(())
}

async fn delete(base: &str, args: DeleteArgs, mode: OutputMode) -> Result<()> {
    if mode == OutputMode::Human && !args.yes {
        let msg = format!("Delete rule '{}'?", args.id);
        if !confirm::confirm_action(&msg) {
            theme::print_dim("  Cancelled.");
            return Ok(());
        }
    }

    let url = format!("{base}/v1/rules/{}", args.id);

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Deleting rule...")),
        OutputMode::Json => None,
    };

    let client = reqwest::Client::new();
    client
        .delete(&url)
        .send()
        .await?
        .error_for_status()?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, &format!("Rule '{}' deleted", args.id));
    }

    if mode == OutputMode::Json {
        print_json(&serde_json::json!({"deleted": true, "id": args.id}))?;
    }

    Ok(())
}
