use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct HistoryArgs {
    pub agent_id: String,
    pub metric: String,

    #[arg(long, default_value = "1h", help = "Lookback window (e.g. 1h, 6h, 1d)")]
    pub last: String,

    #[arg(
        long,
        default_value = "5m",
        help = "Bucket interval (1m, 5m, 15m, 1h, 6h, 1d)"
    )]
    pub interval: String,
}

pub async fn run(args: HistoryArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let (from, to) = parse_range(&args.last)?;
    let path = format!(
        "/v1/metrics/agents/{}/history?metric={}&from={}&to={}&interval={}",
        args.agent_id,
        urlencoding::encode(&args.metric),
        urlencoding::encode(&from),
        urlencoding::encode(&to),
        args.interval,
    );

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching metric history...")),
        OutputMode::Json => None,
    };

    let data = api.get_json(&path).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "History fetched");
    }

    match mode {
        OutputMode::Json => print_json(&data)?,
        OutputMode::Human => render(&args.agent_id, &args.metric, &data),
    }
    Ok(())
}

fn render(agent_id: &str, metric: &str, data: &serde_json::Value) {
    theme::print_header(&format!("{metric} — {agent_id}"));

    let points = match data["points"].as_array() {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            theme::print_dim("No history data for this metric");
            return;
        }
    };

    let values: Vec<f64> = points
        .iter()
        .filter_map(|p| p["avg_value"].as_f64())
        .collect();

    if !values.is_empty() {
        render_sparkline(&values);
        println!();
    }

    let mut tbl = build_table(&["Bucket", "Avg", "Min", "Max", "Samples"]);
    for p in points {
        let bucket = p["bucket"].as_str().unwrap_or("-");
        let avg = fmt_opt(p["avg_value"].as_f64());
        let min = fmt_opt(p["min_value"].as_f64());
        let max = fmt_opt(p["max_value"].as_f64());
        let cnt = p["sample_count"]
            .as_i64()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into());
        tbl.add_row(vec![bucket, &avg, &min, &max, &cnt]);
    }
    println!("{tbl}");
}

fn render_sparkline(values: &[f64]) {
    let sparks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let min = values.iter().cloned().fold(f64::MAX, f64::min);
    let max = values.iter().cloned().fold(f64::MIN, f64::max);
    let range = (max - min).max(1e-9);

    let line: String = values
        .iter()
        .map(|v| {
            let idx = (((v - min) / range) * 7.0).round() as usize;
            sparks[idx.min(7)]
        })
        .collect();

    theme::print_section("Sparkline");
    println!("  {}", line.cyan());
    println!(
        "  {} {:.2}  {} {:.2}",
        "min".dimmed(),
        min,
        "max".dimmed(),
        max
    );
}

fn fmt_opt(v: Option<f64>) -> String {
    v.map(|f| format!("{f:.4}")).unwrap_or_else(|| "-".into())
}

pub(crate) fn parse_range(last: &str) -> Result<(String, String)> {
    let now = chrono::Utc::now();
    let dur = parse_duration(last)?;
    let from = now - dur;
    Ok((from.to_rfc3339(), now.to_rfc3339()))
}

fn parse_duration(s: &str) -> Result<chrono::Duration> {
    let s = s.trim();
    if let Some(h) = s.strip_suffix('h') {
        let n: i64 = h.parse()?;
        return Ok(chrono::Duration::hours(n));
    }
    if let Some(d) = s.strip_suffix('d') {
        let n: i64 = d.parse()?;
        return Ok(chrono::Duration::days(n));
    }
    if let Some(m) = s.strip_suffix('m') {
        let n: i64 = m.parse()?;
        return Ok(chrono::Duration::minutes(n));
    }
    anyhow::bail!("Invalid duration '{s}'. Use e.g. 30m, 1h, 6h, 1d")
}
