use anyhow::Result;

use crate::client;
use crate::output::{bar_chart, print_json, spinner, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching metrics...")),
        OutputMode::Json => None,
    };

    let raw = api.get_text("/metrics").await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Metrics fetched");
    }

    let parsed = parse_prometheus(&raw);

    match mode {
        OutputMode::Json => {
            let obj: serde_json::Value = parsed
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::json!(v)))
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into();
            print_json(&obj)?;
        }
        OutputMode::Human => {
            theme::print_header("Server Metrics");

            let gauges: Vec<(&str, f64)> = parsed
                .iter()
                .filter(|(k, _)| !k.contains("bucket") && !k.contains("_total"))
                .map(|(k, v)| (k.as_str(), *v))
                .collect();

            let counters: Vec<(&str, f64)> = parsed
                .iter()
                .filter(|(k, _)| k.contains("_total"))
                .map(|(k, v)| (k.as_str(), *v))
                .collect();

            if !gauges.is_empty() {
                theme::print_section("Gauges");
                bar_chart::render_metric_bars(&gauges, 30);
            }

            if !counters.is_empty() {
                println!();
                theme::print_section("Counters");
                for (name, value) in &counters {
                    theme::print_kv(name, &format!("{value:.0}"));
                }
            }

            if gauges.is_empty() && counters.is_empty() {
                theme::print_dim("  No metrics available");
            }
        }
    }

    Ok(())
}

fn parse_prometheus(text: &str) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.rsplitn(2, ' ').collect();
        if parts.len() == 2 {
            if let Ok(val) = parts[0].parse::<f64>() {
                out.push((parts[1].to_string(), val));
            }
        }
    }
    out
}
