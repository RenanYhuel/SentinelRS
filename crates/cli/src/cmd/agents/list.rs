use anyhow::Result;
use colored::Colorize;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching agents...")),
        OutputMode::Json => None,
    };

    let agents = api.get_json("/v1/agents").await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&agents)?,
        OutputMode::Human => render(&agents),
    }
    Ok(())
}

fn render(agents: &serde_json::Value) {
    let empty = vec![];
    let arr = agents.as_array().unwrap_or(&empty);
    if arr.is_empty() {
        theme::print_dim("  No agents registered.");
        return;
    }

    theme::print_header("Agents");

    let mut table = build_table(&["Status", "Agent", "Version", "CPU", "Memory", "Latency"]);
    for a in arr {
        table.add_row(vec![
            &status_badge(a),
            a["agent_id"].as_str().unwrap_or("-"),
            a["agent_version"].as_str().unwrap_or("-"),
            &fmt_percent(a["cpu_percent"].as_f64()),
            &fmt_percent(a["memory_percent"].as_f64()),
            &fmt_latency(a["latency_ms"].as_f64()),
        ]);
    }
    println!("{table}");

    let online = arr
        .iter()
        .filter(|a| a["status"].as_str() == Some("online"))
        .count();
    let total = arr.len();
    theme::print_dim(&format!("  {online}/{total} online"));
}

fn status_badge(a: &serde_json::Value) -> String {
    match a["status"].as_str().unwrap_or("offline") {
        "online" => format!("{} Online", "●".green()),
        "stale" => format!("{} Stale", "●".yellow()),
        "bootstrapping" => format!("{} Boot", "◐".cyan()),
        _ => format!("{} Offline", "●".red()),
    }
}

fn fmt_percent(v: Option<f64>) -> String {
    v.map(|p| format!("{p:.1}%")).unwrap_or_else(|| "—".into())
}

fn fmt_latency(v: Option<f64>) -> String {
    v.map(|ms| format!("{ms:.0}ms"))
        .unwrap_or_else(|| "—".into())
}
