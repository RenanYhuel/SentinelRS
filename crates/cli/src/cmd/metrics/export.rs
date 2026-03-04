use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use crate::client;
use crate::output::{print_json, print_success, spinner, OutputMode};

#[derive(Args)]
pub struct ExportArgs {
    pub agent_id: String,

    #[arg(long, default_value = "1h", help = "Lookback window")]
    pub last: String,

    #[arg(long, help = "Filter to a specific metric name")]
    pub metric: Option<String>,

    #[arg(long, default_value = "json", help = "Output format: json or csv")]
    pub format: String,

    #[arg(long, short, help = "Write to file instead of stdout")]
    pub output: Option<PathBuf>,
}

pub async fn run(args: ExportArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let (from, to) = super::history::parse_range(&args.last)?;
    let mut path = format!(
        "/v1/metrics/agents/{}/export?from={}&to={}&format={}",
        args.agent_id,
        urlencoding::encode(&from),
        urlencoding::encode(&to),
        args.format,
    );
    if let Some(ref m) = args.metric {
        path.push_str(&format!("&metric={}", urlencoding::encode(m)));
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Exporting metrics...")),
        OutputMode::Json => None,
    };

    let body = api.get_text(&path).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, "Export complete");
    }

    if let Some(ref out_path) = args.output {
        std::fs::write(out_path, &body)?;
        if mode == OutputMode::Human {
            print_success(&format!("Written to {}", out_path.display()));
        }
    } else if mode == OutputMode::Json && args.format == "json" {
        let val: serde_json::Value = serde_json::from_str(&body)?;
        print_json(&val)?;
    } else {
        println!("{body}");
    }

    Ok(())
}
