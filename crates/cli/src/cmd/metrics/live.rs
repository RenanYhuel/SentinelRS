use anyhow::Result;
use clap::Args;
use std::io::Write;

use crate::client;
use crate::output::{bar_chart, print_error, theme, OutputMode};

#[derive(Args)]
pub struct LiveArgs {
    #[arg(long, default_value = "3")]
    pub interval: u64,
}

pub async fn run(args: LiveArgs, _mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    loop {
        print!("\x1B[2J\x1B[H");
        std::io::stdout().flush()?;

        match api.get_text("/metrics").await {
            Ok(raw) => {
                let parsed = parse_prometheus(&raw);

                theme::print_header("Live Metrics");
                theme::print_dim(&format!(
                    "  Refreshing every {}s — press Ctrl+C to stop",
                    args.interval
                ));
                println!();

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
                    bar_chart::render_metric_bars(&gauges, 40);
                    println!();
                }

                if !counters.is_empty() {
                    theme::print_section("Counters");
                    for (name, value) in &counters {
                        theme::print_kv(name, &format!("{value:.0}"));
                    }
                }
            }
            Err(e) => print_error(&format!("Failed to fetch metrics: {e}")),
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(args.interval)).await;
    }
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
