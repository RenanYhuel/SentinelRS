mod agent;
mod compare;
mod export;
pub(crate) mod history;
mod live;
mod names;
mod percentiles;
mod show;
mod summary;
mod top;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputMode;

#[derive(Subcommand)]
pub enum MetricsCmd {
    #[command(about = "Display server metrics snapshot")]
    Show,
    #[command(about = "Watch metrics in real-time")]
    Live(live::LiveArgs),
    #[command(about = "Show latest metrics for an agent", alias = "a")]
    Agent(agent::AgentArgs),
    #[command(about = "Show metric history with sparkline", alias = "hist")]
    History(history::HistoryArgs),
    #[command(about = "List available metric names for an agent", alias = "ls")]
    Names(names::NamesArgs),
    #[command(about = "Fleet-wide metrics summary", alias = "fleet")]
    Summary,
    #[command(about = "Compare a metric across agents", alias = "cmp")]
    Compare(compare::CompareArgs),
    #[command(about = "Export raw metrics to file", alias = "ex")]
    Export(export::ExportArgs),
    #[command(about = "Top metrics by sample count")]
    Top(top::TopArgs),
    #[command(about = "Percentile analysis for a metric", alias = "pct")]
    Percentiles(percentiles::PercentilesArgs),
}

pub async fn execute(cmd: MetricsCmd, mode: OutputMode, server: Option<String>) -> Result<()> {
    match cmd {
        MetricsCmd::Show => show::run(mode, server).await,
        MetricsCmd::Live(args) => live::run(args, mode, server).await,
        MetricsCmd::Agent(args) => agent::run(args, mode, server).await,
        MetricsCmd::History(args) => history::run(args, mode, server).await,
        MetricsCmd::Names(args) => names::run(args, mode, server).await,
        MetricsCmd::Summary => summary::run(mode, server).await,
        MetricsCmd::Compare(args) => compare::run(args, mode, server).await,
        MetricsCmd::Export(args) => export::run(args, mode, server).await,
        MetricsCmd::Top(args) => top::run(args, mode, server).await,
        MetricsCmd::Percentiles(args) => percentiles::run(args, mode, server).await,
    }
}
