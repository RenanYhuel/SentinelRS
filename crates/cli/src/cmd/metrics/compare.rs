use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::collections::HashMap;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct CompareArgs {
    #[arg(help = "Comma-separated agent IDs")]
    pub agents: String,
    pub metric: String,

    #[arg(long, default_value = "1h", help = "Lookback window")]
    pub last: String,

    #[arg(long, default_value = "5m")]
    pub interval: String,
}

pub async fn run(args: CompareArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let (from, to) = super::history::parse_range(&args.last)?;
    let path = format!(
        "/v1/metrics/compare?agents={}&metric={}&from={}&to={}&interval={}",
        urlencoding::encode(&args.agents),
        urlencoding::encode(&args.metric),
        urlencoding::encode(&from),
        urlencoding::encode(&to),
        args.interval,
    );

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Comparing agents...")),
        OutputMode::Json => None,
    };

    let data = api.get_json(&path).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Comparison fetched");
    }

    match mode {
        OutputMode::Json => print_json(&data)?,
        OutputMode::Human => render(&args.metric, &data),
    }
    Ok(())
}

fn render(metric: &str, data: &serde_json::Value) {
    theme::print_header(&format!("Compare — {metric}"));

    let points = match data.as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            theme::print_dim("No comparison data available");
            return;
        }
    };

    let agents: Vec<String> = points
        .iter()
        .filter_map(|p| p["agent_id"].as_str().map(String::from))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let colors = ["cyan", "green", "yellow", "magenta", "blue", "red"];

    theme::print_section("Agents");
    for (i, agent) in agents.iter().enumerate() {
        let color = colors[i % colors.len()];
        println!("  {} {}", colorize("●", color), agent);
    }
    println!();

    let mut by_bucket: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    for p in points {
        let bucket = p["bucket"].as_str().unwrap_or("-").to_string();
        let agent = p["agent_id"].as_str().unwrap_or("-").to_string();
        let val = p["avg_value"].as_f64().unwrap_or(0.0);
        by_bucket.entry(bucket).or_default().push((agent, val));
    }

    let mut headers = vec!["Bucket"];
    let agent_strs: Vec<String> = agents.iter().map(|a| a.to_string()).collect();
    for a in &agent_strs {
        headers.push(a);
    }
    let mut tbl = build_table(&headers);

    let mut sorted_buckets: Vec<&String> = by_bucket.keys().collect();
    sorted_buckets.sort();

    for bucket in sorted_buckets {
        let entries = &by_bucket[bucket];
        let mut row = vec![bucket.clone()];
        for agent in &agents {
            let val = entries
                .iter()
                .find(|(a, _)| a == agent)
                .map(|(_, v)| format!("{v:.4}"))
                .unwrap_or_else(|| "-".into());
            row.push(val);
        }
        let refs: Vec<&str> = row.iter().map(|s| s.as_str()).collect();
        tbl.add_row(refs);
    }
    println!("{tbl}");
}

fn colorize(text: &str, color: &str) -> String {
    match color {
        "cyan" => text.cyan().to_string(),
        "green" => text.green().to_string(),
        "yellow" => text.yellow().to_string(),
        "magenta" => text.magenta().to_string(),
        "blue" => text.blue().to_string(),
        "red" => text.red().to_string(),
        _ => text.to_string(),
    }
}
