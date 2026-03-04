use anyhow::Result;
use colored::Colorize;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, time_ago, OutputMode};

fn status_label(status: &str, last_seen: Option<&str>) -> String {
    let age = last_seen
        .map(time_ago::format_relative)
        .unwrap_or_else(|| "-".into());
    match status {
        "online" => format!("{} Online ({})", "●".green(), age),
        _ => format!("{} Offline ({})", "●".red(), age),
    }
}

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
            let mut table = build_table(&["Status", "ID", "HW ID", "Version"]);
            for a in arr {
                let status = a["status"].as_str().unwrap_or("offline");
                let last_seen = a["last_seen"].as_str();
                table.add_row(vec![
                    &status_label(status, last_seen),
                    a["agent_id"].as_str().unwrap_or("-"),
                    a["hw_id"].as_str().unwrap_or("-"),
                    a["agent_version"].as_str().unwrap_or("-"),
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}
