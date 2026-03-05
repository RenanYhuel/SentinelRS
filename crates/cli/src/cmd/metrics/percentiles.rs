use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client;
use crate::output::{print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct PercentilesArgs {
    pub agent_id: String,
    pub metric: String,

    #[arg(long, default_value = "1h")]
    pub last: String,
}

pub async fn run(args: PercentilesArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let (from, to) = super::history::parse_range(&args.last)?;
    let path = format!(
        "/v1/metrics/agents/{}/percentiles?metric={}&from={}&to={}",
        args.agent_id,
        urlencoding::encode(&args.metric),
        urlencoding::encode(&from),
        urlencoding::encode(&to),
    );

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Computing percentiles...")),
        OutputMode::Json => None,
    };

    let data = api.get_json(&path).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Percentiles computed");
    }

    match mode {
        OutputMode::Json => print_json(&data)?,
        OutputMode::Human => render(&args.metric, &data),
    }
    Ok(())
}

fn render(metric: &str, data: &serde_json::Value) {
    theme::print_header(&format!("Percentiles — {metric}"));

    let count = data["count"].as_i64().unwrap_or(0);
    if count == 0 {
        theme::print_dim("No data points for this metric");
        return;
    }

    let p50 = data["p50"].as_f64();
    let p90 = data["p90"].as_f64();
    let p99 = data["p99"].as_f64();
    let min = data["min"].as_f64();
    let max = data["max"].as_f64();

    theme::print_section("Distribution");

    if let (Some(mn), Some(mx)) = (min, max) {
        let width = 40;
        let range = (mx - mn).max(1e-9);

        for (label, val) in [("P50", p50), ("P90", p90), ("P99", p99)] {
            if let Some(v) = val {
                let pos = (((v - mn) / range) * width as f64).round() as usize;
                let bar = format!(
                    "{}{}{}",
                    "─".repeat(pos.min(width)),
                    "●".bright_cyan(),
                    "─".repeat(width.saturating_sub(pos + 1))
                );
                println!("  {:<4} {} {:.4}", label.bold(), bar, v);
            }
        }

        println!();
        println!(
            "  {} {:.4}  {} {:.4}",
            "min".dimmed(),
            mn,
            "max".dimmed(),
            mx
        );
    }

    println!();
    theme::print_kv("Sample count", &count.to_string());
}
