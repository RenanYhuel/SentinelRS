use anyhow::Result;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, time_ago, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching agents...")),
        OutputMode::Json => None,
    };

    let agents = api.get_json("/v1/agents").await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&agents)?,
        OutputMode::Human => {
            let empty = vec![];
            let arr = agents.as_array().unwrap_or(&empty);
            if arr.is_empty() {
                theme::print_dim("  No agents registered.");
                return Ok(());
            }
            theme::print_header("Agents");
            let mut table = build_table(&["ID", "HW ID", "Version", "Last Seen"]);
            for a in arr {
                table.add_row(vec![
                    a["agent_id"].as_str().unwrap_or("-"),
                    a["hw_id"].as_str().unwrap_or("-"),
                    a["agent_version"].as_str().unwrap_or("-"),
                    &a["last_seen"]
                        .as_str()
                        .map(time_ago::format_relative)
                        .unwrap_or_else(|| "never".into()),
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}
