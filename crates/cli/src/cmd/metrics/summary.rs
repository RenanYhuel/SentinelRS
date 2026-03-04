use anyhow::Result;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, time_ago, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching fleet summary...")),
        OutputMode::Json => None,
    };

    let data = api.get_json("/v1/metrics/summary").await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Summary fetched");
    }

    match mode {
        OutputMode::Json => print_json(&data)?,
        OutputMode::Human => render(&data),
    }
    Ok(())
}

fn render(data: &serde_json::Value) {
    theme::print_header("Fleet Metrics Summary");

    let items = match data.as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            theme::print_dim("No agent metrics found");
            return;
        }
    };

    let mut tbl = build_table(&["Agent", "Metrics", "Samples", "Last Activity"]);
    let mut total_metrics: i64 = 0;
    let mut total_samples: i64 = 0;

    for item in items {
        let agent = item["agent_id"].as_str().unwrap_or("-");
        let metrics = item["metric_count"].as_i64().unwrap_or(0);
        let samples = item["sample_count"].as_i64().unwrap_or(0);
        total_metrics += metrics;
        total_samples += samples;
        let seen = item["latest_time"]
            .as_str()
            .map(time_ago::format_relative)
            .unwrap_or_else(|| "-".into());
        tbl.add_row(vec![
            agent,
            &metrics.to_string(),
            &samples.to_string(),
            &seen,
        ]);
    }
    println!("{tbl}");

    println!();
    theme::print_section("Totals");
    theme::print_kv("Agents", &items.len().to_string());
    theme::print_kv("Distinct metrics", &total_metrics.to_string());
    theme::print_kv("Total samples", &total_samples.to_string());
}
