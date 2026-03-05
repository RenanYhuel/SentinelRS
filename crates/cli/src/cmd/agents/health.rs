use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client;
use crate::output::{bar_chart, print_json, spinner, theme, time_ago, OutputMode};

use super::get::pick_agent;

#[derive(Args)]
pub struct HealthArgs {
    #[arg(help = "Agent ID (omit for interactive selection)")]
    pub id: Option<String>,
}

pub async fn run(args: HealthArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let agent_id = match args.id {
        Some(id) => id,
        None => pick_agent(&api).await?,
    };

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching health...")),
        OutputMode::Json => None,
    };

    let health = api
        .get_json(&format!("/v1/agents/{agent_id}/health"))
        .await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&health)?,
        OutputMode::Human => render(&health),
    }
    Ok(())
}

fn render(h: &serde_json::Value) {
    theme::print_header("Agent Health");

    theme::print_kv("Agent", h["agent_id"].as_str().unwrap_or("-"));
    theme::print_kv("Status", &status_badge(h));

    if let Some(q) = h["connection_quality"].as_str() {
        theme::print_kv("Quality", &quality_colored(q));
    }
    if let Some(since) = h["connected_since"].as_str() {
        theme::print_kv("Connected", &time_ago::format_relative(since));
    }
    if let Some(hb) = h["heartbeat_count"].as_u64() {
        theme::print_kv("Heartbeats", &hb.to_string());
    }

    render_latency(&h["latency"]);
    render_system(&h["system"]);

    if let Some(caps) = h["capabilities"].as_array() {
        if !caps.is_empty() {
            theme::divider();
            theme::print_section("Capabilities");
            for c in caps {
                if let Some(s) = c.as_str() {
                    println!("  • {s}");
                }
            }
        }
    }
    println!();
}

fn render_latency(lat: &serde_json::Value) {
    if lat.is_null() {
        return;
    }
    theme::divider();
    theme::print_section("Latency");

    let pairs = [
        ("Avg", "avg_ms"),
        ("Min", "min_ms"),
        ("Max", "max_ms"),
        ("P50", "p50_ms"),
        ("P95", "p95_ms"),
        ("P99", "p99_ms"),
        ("Jitter", "jitter_ms"),
    ];

    for (label, key) in pairs {
        if let Some(v) = lat[key].as_f64() {
            theme::print_kv(label, &format!("{v:.1} ms"));
        }
    }

    if let Some(n) = lat["sample_count"].as_u64() {
        theme::print_kv("Samples", &n.to_string());
    }
}

fn render_system(sys: &serde_json::Value) {
    if sys.is_null() {
        return;
    }
    theme::divider();
    theme::print_section("System");

    if let Some(h) = sys["hostname"].as_str() {
        theme::print_kv("Hostname", h);
    }
    if let Some(os) = sys["os_name"].as_str() {
        theme::print_kv("OS", os);
    }
    if let Some(up) = sys["uptime_seconds"].as_u64() {
        theme::print_kv("Uptime", &format_uptime(up));
    }
    if let Some(load) = sys["load_avg_1m"].as_f64() {
        theme::print_kv("Load 1m", &format!("{load:.2}"));
    }
    if let Some(procs) = sys["process_count"].as_u64() {
        theme::print_kv("Processes", &procs.to_string());
    }

    let mut bars: Vec<(&str, f64)> = Vec::new();
    if let Some(v) = sys["cpu_percent"].as_f64() {
        bars.push(("CPU", v));
    }
    if let Some(v) = sys["memory_percent"].as_f64() {
        bars.push(("Memory", v));
    }
    if let Some(v) = sys["disk_percent"].as_f64() {
        bars.push(("Disk", v));
    }
    if !bars.is_empty() {
        println!();
        bar_chart::render_metric_bars(&bars, 40);
    }
}

fn status_badge(h: &serde_json::Value) -> String {
    match h["status"].as_str().unwrap_or("offline") {
        "online" => format!("{} Online", "●".green()),
        "stale" => format!("{} Stale", "●".yellow()),
        _ => format!("{} Offline", "●".red()),
    }
}

fn quality_colored(q: &str) -> String {
    match q {
        "Excellent" => q.green().to_string(),
        "Good" => q.blue().to_string(),
        "Fair" => q.yellow().to_string(),
        _ => q.red().to_string(),
    }
}

fn format_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}
