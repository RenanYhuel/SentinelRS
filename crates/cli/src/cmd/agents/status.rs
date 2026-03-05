use anyhow::Result;
use colored::Colorize;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching fleet overview...")),
        OutputMode::Json => None,
    };

    let fleet = api.get_json("/v1/fleet/overview").await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&fleet)?,
        OutputMode::Human => render(&fleet),
    }
    Ok(())
}

fn render(f: &serde_json::Value) {
    theme::print_header("Fleet Overview");

    let total = f["total"].as_u64().unwrap_or(0);
    let online = f["online"].as_u64().unwrap_or(0);
    let offline = f["offline"].as_u64().unwrap_or(0);
    let stale = f["stale"].as_u64().unwrap_or(0);
    let boot = f["bootstrapping"].as_u64().unwrap_or(0);

    theme::print_kv("Total", &total.to_string());
    theme::print_kv_colored("Online", &online.to_string(), true);
    if offline > 0 {
        theme::print_kv_colored("Offline", &offline.to_string(), false);
    }
    if stale > 0 {
        theme::print_kv_colored("Stale", &stale.to_string(), false);
    }
    if boot > 0 {
        theme::print_kv_colored("Bootstrapping", &boot.to_string(), false);
    }

    if let Some(cpu) = f["avg_cpu_percent"].as_f64() {
        theme::print_kv("Avg CPU", &format!("{cpu:.1}%"));
    }
    if let Some(mem) = f["avg_memory_percent"].as_f64() {
        theme::print_kv("Avg Memory", &format!("{mem:.1}%"));
    }
    if let Some(lat) = f["avg_latency_ms"].as_f64() {
        theme::print_kv("Avg Latency", &format!("{lat:.0} ms"));
    }

    let empty = vec![];
    let agents = f["agents"].as_array().unwrap_or(&empty);
    if agents.is_empty() {
        return;
    }

    theme::divider();
    let mut table = build_table(&["Status", "Agent", "CPU", "Memory", "Latency", "Host"]);
    for a in agents {
        table.add_row(vec![
            &status_icon(a),
            a["agent_id"].as_str().unwrap_or("-"),
            &fmt_pct(a["cpu_percent"].as_f64()),
            &fmt_pct(a["memory_percent"].as_f64()),
            &fmt_lat(a["latency_ms"].as_f64()),
            a["hostname"].as_str().unwrap_or("—"),
        ]);
    }
    println!("{table}");
}

fn status_icon(a: &serde_json::Value) -> String {
    match a["status"].as_str().unwrap_or("offline") {
        "online" => "●".green().to_string(),
        "stale" => "●".yellow().to_string(),
        "bootstrapping" => "◐".cyan().to_string(),
        _ => "●".red().to_string(),
    }
}

fn fmt_pct(v: Option<f64>) -> String {
    v.map(|p| format!("{p:.1}%")).unwrap_or_else(|| "—".into())
}

fn fmt_lat(v: Option<f64>) -> String {
    v.map(|ms| format!("{ms:.0}ms"))
        .unwrap_or_else(|| "—".into())
}
