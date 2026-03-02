use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{confirm, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct DeleteArgs {
    #[arg(help = "Agent ID (omit for interactive selection)")]
    pub id: Option<String>,

    #[arg(long, help = "Skip confirmation prompt")]
    pub yes: bool,
}

pub async fn run(args: DeleteArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let agent_id = match args.id {
        Some(id) => id,
        None => super::get::pick_agent(&api).await?,
    };

    if mode == OutputMode::Human && !args.yes {
        let msg = format!("Delete agent '{agent_id}'? This cannot be undone.");
        if !confirm::confirm_action(&msg) {
            theme::print_dim("  Cancelled.");
            return Ok(());
        }
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Deleting agent...")),
        OutputMode::Json => None,
    };

    let status = api.delete_path(&format!("/v1/agents/{agent_id}")).await?;

    if let Some(sp) = sp {
        if status.is_success() || status.as_u16() == 204 {
            spinner::finish_ok(&sp, &format!("Agent '{agent_id}' deleted"));
        } else {
            spinner::finish_err(&sp, &format!("Delete failed (HTTP {status})"));
        }
    }

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "deleted": status.is_success() || status.as_u16() == 204,
            "agent_id": agent_id,
        }))?,
        OutputMode::Human => {}
    }

    Ok(())
}
