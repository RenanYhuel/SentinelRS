use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{bar_chart, build_table, print_json, spinner, theme, time_ago, OutputMode};

#[derive(Args)]
pub struct TopArgs {
    pub agent_id: String,

    #[arg(long, default_value = "20")]
    pub limit: u32,
}

pub async fn run(args: TopArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;
    let path = format!(
        "/v1/metrics/agents/{}/top?limit={}",
        args.agent_id, args.limit
    );

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching top metrics...")),
        OutputMode::Json => None,
    };

    let data = api.get_json(&path).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Top metrics fetched");
    }

    match mode {
        OutputMode::Json => print_json(&data)?,
        OutputMode::Human => render(&args.agent_id, &data),
    }
    Ok(())
}

fn render(agent_id: &str, data: &serde_json::Value) {
    theme::print_header(&format!("Top Metrics — {agent_id}"));

    let items = match data["top"].as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            theme::print_dim("No metrics found");
            return;
        }
    };

    let bars: Vec<(&str, f64)> = items
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?;
            let count = m["total_samples"].as_i64().unwrap_or(0) as f64;
            Some((name, count))
        })
        .collect();

    if !bars.is_empty() {
        theme::print_section("Sample Distribution");
        bar_chart::render_metric_bars(&bars, 35);
        println!();
    }

    let mut tbl = build_table(&["#", "Metric", "Samples", "Last Seen"]);
    for (i, item) in items.iter().enumerate() {
        let rank = (i + 1).to_string();
        let name = item["name"].as_str().unwrap_or("-");
        let samples = item["total_samples"]
            .as_i64()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into());
        let seen = item["last_seen"]
            .as_str()
            .map(time_ago::format_relative)
            .unwrap_or_else(|| "-".into());
        tbl.add_row(vec![&rank, name, &samples, &seen]);
    }
    println!("{tbl}");
}
