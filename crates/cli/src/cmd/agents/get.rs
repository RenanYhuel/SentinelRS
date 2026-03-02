use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{print_json, select, spinner, theme, OutputMode};

#[derive(Args)]
pub struct GetArgs {
    #[arg(help = "Agent ID (omit for interactive selection)")]
    pub id: Option<String>,
}

pub async fn run(args: GetArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let agent_id = match args.id {
        Some(id) => id,
        None => pick_agent(&api).await?,
    };

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching agent...")),
        OutputMode::Json => None,
    };

    let agent = api.get_json(&format!("/v1/agents/{agent_id}")).await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&agent)?,
        OutputMode::Human => {
            theme::print_header("Agent Details");
            for (k, v) in agent.as_object().into_iter().flatten() {
                theme::print_kv(k, &v.to_string());
            }
            println!();
        }
    }

    Ok(())
}

pub async fn pick_agent(api: &client::ApiClient) -> Result<String> {
    let agents = api.get_json("/v1/agents").await?;
    let arr = agents
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected response"))?;

    if arr.is_empty() {
        anyhow::bail!("no agents found");
    }

    let labels: Vec<String> = arr
        .iter()
        .map(|a| {
            format!(
                "{} ({})",
                a["agent_id"].as_str().unwrap_or("?"),
                a["hw_id"].as_str().unwrap_or("?")
            )
        })
        .collect();

    let idx = select::fuzzy_select("Select agent", &labels)
        .ok_or_else(|| anyhow::anyhow!("cancelled"))?;

    Ok(arr[idx]["agent_id"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}
