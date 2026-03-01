mod cmd;
mod output;
#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::Parser;
use cmd::Commands;
use output::OutputMode;

#[derive(Parser)]
#[command(name = "sentinel", version, about = "SentinelRS Admin CLI")]
pub struct Opts {
    #[clap(subcommand)]
    cmd: Commands,

    #[arg(long, global = true, help = "Output as JSON")]
    json: bool,

    #[arg(long, global = true, help = "Server base URL (overrides config)")]
    server: Option<String>,

    #[arg(long, global = true, help = "Path to agent config file")]
    config: Option<String>,
}

impl Opts {
    pub fn output_mode(&self) -> OutputMode {
        if self.json {
            OutputMode::Json
        } else {
            OutputMode::Human
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    cmd::run(opts).await
}
