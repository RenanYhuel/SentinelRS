use anyhow::{Context, Result};
use clap::Subcommand;

use crate::output::{OutputMode, print_json, print_success, build_table, spinner, theme};
use super::helpers;

#[derive(Subcommand)]
pub enum AgentsCmd {
    List,
    Get(GetArgs),
}

#[derive(clap::Args)]
pub struct GetArgs {
    #[arg(help = "Agent ID")]
    id: String,
}

pub async fn execute(
    cmd: AgentsCmd,
    mode: OutputMode,
    server: Option<String>,
    config_path: Option<String>,
) -> Result<()> {
    match cmd {
        AgentsCmd::List => list(mode, server, config_path).await,
        AgentsCmd::Get(args) => get(args, mode, server, config_path).await,
    }
}

async fn list(
    mode: OutputMode,
    server: Option<String>,
    config_path: Option<String>,
) -> Result<()> {
    let base = helpers::resolve_rest_url(server.as_deref(), config_path.as_deref())?;
    let url = format!("{base}/v1/agents");

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching agents...")),
        OutputMode::Json => None,
    };

    let resp = reqwest::get(&url)
        .await
        .context("failed to reach server")?
        .error_for_status()
        .context("server returned error")?;

    let agents: Vec<serde_json::Value> = resp.json().await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&agents)?,
        OutputMode::Human => {
            if agents.is_empty() {
                print_success("No agents registered");
                return Ok(());
            }
            theme::print_header("Agents");
            let mut table = build_table(&["Agent ID", "HW ID", "Version", "Last Seen"]);
            for a in &agents {
                table.add_row(vec![
                    a["agent_id"].as_str().unwrap_or("-"),
                    a["hw_id"].as_str().unwrap_or("-"),
                    a["agent_version"].as_str().unwrap_or("-"),
                    a["last_heartbeat"].as_str().unwrap_or("-"),
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}

async fn get(
    args: GetArgs,
    mode: OutputMode,
    server: Option<String>,
    config_path: Option<String>,
) -> Result<()> {
    let base = helpers::resolve_rest_url(server.as_deref(), config_path.as_deref())?;
    let url = format!("{base}/v1/agents/{}", args.id);

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching agent details...")),
        OutputMode::Json => None,
    };

    let resp = reqwest::get(&url)
        .await
        .context("failed to reach server")?
        .error_for_status()
        .context("agent not found or server error")?;

    let agent: serde_json::Value = resp.json().await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&agent)?,
        OutputMode::Human => {
            theme::print_header("Agent Details");
            theme::print_kv("Agent ID", agent["agent_id"].as_str().unwrap_or("-"));
            theme::print_kv("HW ID", agent["hw_id"].as_str().unwrap_or("-"));
            theme::print_kv("Version", agent["agent_version"].as_str().unwrap_or("-"));
            theme::print_kv("Last Seen", agent["last_heartbeat"].as_str().unwrap_or("-"));
            println!();
        }
    }

    Ok(())
}
