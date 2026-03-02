use anyhow::Result;
use colored::Colorize;

use crate::client::{self, sse};
use crate::output::{print_json, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;
    let url = api.streaming_url("/v1/cluster/events");

    if mode == OutputMode::Human {
        theme::print_header("Cluster Events");
        println!(
            "  {} Streaming events (Ctrl+C to stop)\n",
            "◉".bright_cyan()
        );
    }

    sse::stream_events(api.streaming_client(), &url, |event| {
        match mode {
            OutputMode::Json => {
                let _ = print_json(&serde_json::json!({
                    "event": event.event,
                    "data": serde_json::from_str::<serde_json::Value>(&event.data)
                        .unwrap_or(serde_json::Value::String(event.data.clone())),
                }));
            }
            OutputMode::Human => {
                let icon = match event.event.as_str() {
                    "agent_connected" => "↑".green().bold().to_string(),
                    "agent_disconnected" => "↓".red().bold().to_string(),
                    "agent_stale" => "⚠".yellow().bold().to_string(),
                    "heartbeat" => "♥".dimmed().to_string(),
                    _ => "●".dimmed().to_string(),
                };
                let ts = chrono::Local::now().format("%H:%M:%S");
                println!(
                    "  {} {} {} {}",
                    ts.to_string().dimmed(),
                    icon,
                    event.event.bright_white(),
                    event.data.dimmed()
                );
            }
        }
        true
    })
    .await
}
