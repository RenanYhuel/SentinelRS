use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{bar_chart, build_table, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct AgentArgs {
    pub agent_id: String,
}

pub async fn run(args: AgentArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;
    let path = format!("/v1/metrics/agents/{}/latest", args.agent_id);

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching agent metrics...")),
        OutputMode::Json => None,
    };

    let data = api.get_json(&path).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Metrics fetched");
    }

    match mode {
        OutputMode::Json => print_json(&data)?,
        OutputMode::Human => render(&args.agent_id, &data),
    }
    Ok(())
}

fn render(agent_id: &str, data: &serde_json::Value) {
    theme::print_header(&format!("Metrics — {agent_id}"));

    let metrics = &data["metrics"];
    let items = match metrics.as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            theme::print_dim("No metrics available for this agent");
            return;
        }
    };

    let bars: Vec<(&str, f64)> = items
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?;
            let val = m["value"].as_f64().unwrap_or(0.0);
            Some((name, val))
        })
        .collect();

    if !bars.is_empty() {
        theme::print_section("Current Values");
        bar_chart::render_metric_bars(&bars, 35);
        println!();
    }

    let mut tbl = build_table(&["Metric", "Value", "Last Seen"]);
    for m in items {
        let name = m["name"].as_str().unwrap_or("-");
        let val = m["value"]
            .as_f64()
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "-".into());
        let time = m["time"].as_str().unwrap_or("-");
        tbl.add_row(vec![name, &val, time]);
    }
    println!("{tbl}");
}
