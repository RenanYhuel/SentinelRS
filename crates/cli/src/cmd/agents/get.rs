use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client;
use crate::output::{bar_chart, print_json, select, spinner, theme, time_ago, OutputMode};

#[derive(Args)]
pub struct GetArgs {
    #[arg(help = "Agent ID (omit for interactive selection)")]
    pub id: Option<String>,
}

pub async fn run(args: GetArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let agent_id = match args.id {
        Some(id) => id,
        None => pick_agent(&api).await?,
    };

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching agent...")),
        OutputMode::Json => None,
    };

    let agent = api.get_json(&format!("/v1/agents/{agent_id}")).await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&agent)?,
        OutputMode::Human => render(&agent),
    }
    Ok(())
}

fn render(a: &serde_json::Value) {
    theme::print_header("Agent Details");

    theme::print_kv("Status", &status_badge(a));
    theme::print_kv("Agent ID", str_or(a, "agent_id"));
    theme::print_kv("HW ID", str_or(a, "hw_id"));
    theme::print_kv("Version", str_or(a, "agent_version"));
    theme::print_kv("Last Seen", &last_seen(a));

    let s = a["status"].as_str().unwrap_or("offline");
    if s == "online" || s == "stale" {
        render_session(a);
    }

    println!();
}

fn render_session(a: &serde_json::Value) {
    theme::divider();
    theme::print_section("Session");

    if let Some(host) = a["hostname"].as_str() {
        theme::print_kv("Hostname", host);
    }
    if let Some(os) = a["os_name"].as_str() {
        theme::print_kv("OS", os);
    }
    if let Some(since) = a["connected_since"].as_str() {
        theme::print_kv("Connected", &time_ago::format_relative(since));
    }
    if let Some(q) = a["connection_quality"].as_str() {
        theme::print_kv("Quality", &quality_colored(q));
    }
    if let Some(lat) = a["latency_ms"].as_f64() {
        theme::print_kv("Latency", &format!("{lat:.0} ms"));
    }
    if let Some(up) = a["uptime_seconds"].as_u64() {
        theme::print_kv("Uptime", &format_uptime(up));
    }
    if let Some(hb) = a["heartbeat_count"].as_u64() {
        theme::print_kv("Heartbeats", &hb.to_string());
    }

    render_resource_bars(a);
}

fn render_resource_bars(a: &serde_json::Value) {
    let mut bars: Vec<(&str, f64)> = Vec::new();
    if let Some(v) = a["cpu_percent"].as_f64() {
        bars.push(("CPU", v));
    }
    if let Some(v) = a["memory_percent"].as_f64() {
        bars.push(("Memory", v));
    }
    if let Some(v) = a["disk_percent"].as_f64() {
        bars.push(("Disk", v));
    }
    if !bars.is_empty() {
        println!();
        bar_chart::render_metric_bars(&bars, 40);
    }
}

fn status_badge(a: &serde_json::Value) -> String {
    match a["status"].as_str().unwrap_or("offline") {
        "online" => format!("{} Online", "●".green()),
        "stale" => format!("{} Stale", "●".yellow()),
        "bootstrapping" => format!("{} Bootstrapping", "◐".cyan()),
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

fn str_or<'a>(v: &'a serde_json::Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("-")
}

fn last_seen(a: &serde_json::Value) -> String {
    a["last_seen"]
        .as_str()
        .map(time_ago::format_relative)
        .unwrap_or_else(|| "never".into())
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

pub async fn pick_agent(api: &client::ApiClient) -> Result<String> {
    let agents = api.get_json("/v1/agents").await?;
    let arr = agents
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected response"))?;

    if arr.is_empty() {
        anyhow::bail!("no agents found");
    }

    let labels: Vec<String> = arr
        .iter()
        .map(|a| {
            let status = match a["status"].as_str().unwrap_or("offline") {
                "online" => "●",
                "stale" => "◐",
                _ => "○",
            };
            format!(
                "{status} {} ({})",
                a["agent_id"].as_str().unwrap_or("?"),
                a["hostname"]
                    .as_str()
                    .unwrap_or(a["hw_id"].as_str().unwrap_or("?"))
            )
        })
        .collect();

    let idx = select::fuzzy_select("Select agent", &labels)
        .ok_or_else(|| anyhow::anyhow!("cancelled"))?;

    Ok(arr[idx]["agent_id"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}
