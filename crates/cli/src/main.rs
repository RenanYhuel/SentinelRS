mod client;
mod cmd;
mod output;
mod store;
#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::Parser;
use cmd::Commands;
use output::OutputMode;

#[derive(Parser)]
#[command(
    name = "sentinel",
    version,
    about = "SentinelRS CLI — manage agents, rules, metrics and more"
)]
pub struct Opts {
    #[clap(subcommand)]
    cmd: Commands,

    #[arg(long, global = true, help = "Output as JSON")]
    json: bool,

    #[arg(long, global = true, help = "Server URL (overrides stored config)")]
    server: Option<String>,

    #[arg(long, global = true, help = "Path to agent config file")]
    config: Option<String>,
}

impl Opts {
    pub fn output_mode(&self) -> OutputMode {
        if self.json {
            return OutputMode::Json;
        }
        if let Ok(cfg) = store::load() {
            if cfg.output() == "json" {
                return OutputMode::Json;
            }
        }
        OutputMode::Human
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    cmd::run(opts).await
}
