use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client;
use crate::output::{print_json, theme, OutputMode};

#[derive(Args)]
pub struct LiveArgs {
    #[arg(help = "Agent ID (omit for interactive selection)")]
    pub id: Option<String>,

    #[arg(long, default_value = "2", help = "Refresh interval in seconds")]
    pub interval: u64,
}

pub async fn run(args: LiveArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let agent_id = match args.id {
        Some(id) => id,
        None => super::get::pick_agent(&api).await?,
    };

    if mode == OutputMode::Json {
        let snap = api.get_json(&format!("/v1/agents/{agent_id}/live")).await?;
        return print_json(&snap);
    }

    println!();
    println!(
        "  {} Watching agent {} (Ctrl+C to stop)",
        "◉".bright_cyan(),
        agent_id.bright_white().bold()
    );
    theme::divider();

    let interval = std::time::Duration::from_secs(args.interval);

    loop {
        match api.get_json(&format!("/v1/agents/{agent_id}/live")).await {
            Ok(snap) => render_snapshot(&snap),
            Err(_) => {
                println!("  {} agent not connected", "●".red());
            }
        }

        tokio::time::sleep(interval).await;
        clear_previous_render();
    }
}

fn render_snapshot(snap: &serde_json::Value) {
    let fields = [
        ("agent_id", "Agent ID"),
        ("agent_version", "Version"),
        ("last_ping", "Last Heartbeat"),
        ("heartbeat_count", "Heartbeats"),
        ("connected_at", "Connected Since"),
        ("connection_quality", "Quality"),
        ("memory_percent", "Memory %"),
    ];
    for (key, label) in &fields {
        if let Some(val) = snap.get(key) {
            theme::print_kv(label, &val.to_string());
        }
    }
    println!();
}

fn clear_previous_render() {
    use std::io::Write;
    print!("\x1b[8A\x1b[J");
    std::io::stdout().flush().ok();
}
