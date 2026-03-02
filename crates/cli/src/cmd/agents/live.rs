use anyhow::Result;
use clap::Args;
use colored::Colorize;
use crossterm::{cursor, execute, terminal};

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
    let mut prev_lines: u16 = 0;

    loop {
        let lines = match api.get_json(&format!("/v1/agents/{agent_id}/live")).await {
            Ok(snap) => render_snapshot(&snap),
            Err(_) => {
                println!("  {} agent not connected", "●".red());
                println!();
                2
            }
        };

        tokio::time::sleep(interval).await;

        if prev_lines > 0 {
            let mut stdout = std::io::stdout();
            let _ = execute!(
                stdout,
                cursor::MoveUp(prev_lines),
                terminal::Clear(terminal::ClearType::FromCursorDown)
            );
        }
        prev_lines = lines as u16;
    }
}

const FIELDS: &[(&str, &str)] = &[
    ("agent_id", "Agent ID"),
    ("agent_version", "Version"),
    ("last_ping", "Last Heartbeat"),
    ("heartbeat_count", "Heartbeats"),
    ("connected_at", "Connected Since"),
    ("connection_quality", "Quality"),
    ("memory_percent", "Memory %"),
];

fn render_snapshot(snap: &serde_json::Value) -> usize {
    let mut count = 0;
    for (key, label) in FIELDS {
        let display = match snap.get(key) {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => "-".into(),
        };
        theme::print_kv(label, &display);
        count += 1;
    }
    println!();
    count + 1
}
