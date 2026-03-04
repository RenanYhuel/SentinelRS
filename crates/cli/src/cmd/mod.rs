mod agents;
mod alerts;
mod cluster;
mod completions;
mod config;
mod doctor;
mod force_send;
mod health;
mod init;
mod key;
mod metrics;
mod notifiers;
mod register;
mod rules;
mod status;
mod version;
pub(crate) mod wal;

use anyhow::Result;
use clap::Subcommand;
use clap_complete::Shell;

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Interactive setup wizard")]
    Init,

    #[command(about = "Run system diagnostics")]
    Doctor,

    #[command(about = "Generate shell completions")]
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },

    #[command(subcommand, about = "Manage registered agents")]
    Agents(agents::AgentsCmd),

    #[command(subcommand, about = "View and filter alerts")]
    Alerts(alerts::AlertsCmd),

    #[command(subcommand, about = "Cluster status and monitoring")]
    Cluster(cluster::ClusterCmd),

    #[command(subcommand, about = "CLI configuration")]
    Config(config::ConfigCmd),

    #[command(subcommand, about = "Alert rule management")]
    Rules(rules::RulesCmd),

    #[command(subcommand, about = "Notifier management", visible_alias = "notify")]
    Notifiers(notifiers::NotifiersCmd),

    #[command(subcommand, about = "Encryption key management")]
    Key(key::KeyCmd),

    #[command(subcommand, about = "WAL inspection and maintenance")]
    Wal(wal::WalCmd),

    #[command(subcommand, about = "Server metrics visualization")]
    Metrics(metrics::MetricsCmd),

    #[command(about = "Check server health")]
    Health,

    #[command(about = "Show agent and server status")]
    Status,

    #[command(about = "Register a new agent via gRPC")]
    Register(register::RegisterArgs),

    #[command(about = "Force-send unacked WAL batches")]
    ForceSend(force_send::ForceSendArgs),

    #[command(about = "Show CLI version")]
    Version,
}

pub async fn run(opts: crate::Opts) -> Result<()> {
    let mode = opts.output_mode();
    match opts.cmd {
        Commands::Init => init::execute(mode).await,
        Commands::Doctor => doctor::execute(mode, opts.server).await,
        Commands::Completions { shell } => completions::execute(shell),
        Commands::Agents(cmd) => agents::execute(cmd, mode, opts.server).await,
        Commands::Alerts(cmd) => alerts::execute(cmd, mode, opts.server).await,
        Commands::Cluster(cmd) => cluster::execute(cmd, mode, opts.server).await,
        Commands::Config(cmd) => config::execute(cmd, mode).await,
        Commands::Rules(cmd) => rules::execute(cmd, mode, opts.server).await,
        Commands::Notifiers(cmd) => notifiers::execute(cmd, mode, opts.server).await,
        Commands::Key(cmd) => {
            key::execute(cmd, mode, opts.config)?;
            Ok(())
        }
        Commands::Wal(cmd) => {
            wal::execute(cmd, mode, opts.config)?;
            Ok(())
        }
        Commands::Metrics(cmd) => metrics::execute(cmd, mode, opts.server).await,
        Commands::Health => health::run(mode, opts.server).await,
        Commands::Status => status::run(mode, opts.server, opts.config).await,
        Commands::Register(args) => register::run(args, mode, opts.server).await,
        Commands::ForceSend(args) => force_send::run(args, mode, opts.server, opts.config).await,
        Commands::Version => {
            version::run(mode);
            Ok(())
        }
    }
}
