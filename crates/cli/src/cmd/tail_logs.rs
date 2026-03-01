use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Args)]
pub struct TailLogsArgs {
    #[arg(long, help = "Path to log file")]
    file: Option<String>,

    #[arg(long, default_value = "20")]
    lines: usize,

    #[arg(long, help = "Follow the log output")]
    follow: bool,
}

pub async fn execute(args: TailLogsArgs) -> Result<()> {
    let path = args
        .file
        .map(PathBuf::from)
        .unwrap_or_else(default_log_path);

    if !path.exists() {
        anyhow::bail!("log file not found: {}", path.display());
    }

    let content = tokio::fs::read_to_string(&path).await?;
    let all_lines: Vec<&str> = content.lines().collect();
    let start = all_lines.len().saturating_sub(args.lines);

    for line in &all_lines[start..] {
        println!("{}", colorize_log_line(line));
    }

    if args.follow {
        let file = tokio::fs::File::open(&path).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let skip = all_lines.len();
        let mut current = 0;

        while let Some(line) = lines.next_line().await? {
            if current >= skip {
                println!("{}", colorize_log_line(&line));
            }
            current += 1;
        }

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let file = tokio::fs::File::open(&path).await?;
            let reader = BufReader::new(file);
            let mut lines = reader.lines();
            let mut idx = 0;
            while let Some(line) = lines.next_line().await? {
                if idx >= current {
                    println!("{}", colorize_log_line(&line));
                    current = idx + 1;
                }
                idx += 1;
            }
        }
    }

    Ok(())
}

fn colorize_log_line(line: &str) -> String {
    let lower = line.to_lowercase();
    if lower.contains("error") || lower.contains("err]") || lower.contains("fatal") {
        line.red().bold().to_string()
    } else if lower.contains("warn") {
        line.yellow().to_string()
    } else if lower.contains("info") {
        line.cyan().to_string()
    } else if lower.contains("debug") || lower.contains("trace") {
        line.dimmed().to_string()
    } else {
        line.to_string()
    }
}

fn default_log_path() -> PathBuf {
    if cfg!(windows) {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel")
            .join("agent.log")
    } else {
        PathBuf::from("/var/log/sentinel/agent.log")
    }
}
