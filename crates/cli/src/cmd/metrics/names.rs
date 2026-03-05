use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, time_ago, OutputMode};

#[derive(Args)]
pub struct NamesArgs {
    pub agent_id: String,
}

pub async fn run(args: NamesArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;
    let path = format!("/v1/metrics/agents/{}/names", args.agent_id);

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching metric names...")),
        OutputMode::Json => None,
    };

    let data = api.get_json(&path).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Names fetched");
    }

    match mode {
        OutputMode::Json => print_json(&data)?,
        OutputMode::Human => render(&args.agent_id, &data),
    }
    Ok(())
}

fn render(agent_id: &str, data: &serde_json::Value) {
    theme::print_header(&format!("Metric Names — {agent_id}"));

    let items = match data["names"].as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            theme::print_dim("No metrics found for this agent");
            return;
        }
    };

    let mut tbl = build_table(&["Name", "Samples", "Last Seen"]);
    for item in items {
        let name = item["name"].as_str().unwrap_or("-");
        let samples = item["total_samples"]
            .as_i64()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into());
        let seen = item["last_seen"]
            .as_str()
            .map(time_ago::format_relative)
            .unwrap_or_else(|| "-".into());
        tbl.add_row(vec![name, &samples, &seen]);
    }
    println!("{tbl}");
    theme::print_dim(&format!("  {} metrics total", items.len()));
}
