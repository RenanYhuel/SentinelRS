mod agents;
mod config;
mod force_send;
mod health;
pub(crate) mod helpers;
mod key;
mod notifiers;
mod register;
mod rules;
mod status;
mod tail_logs;
mod version;
mod wal;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Register(register::RegisterArgs),
    #[command(subcommand)]
    Config(config::ConfigCmd),
    #[command(subcommand)]
    Wal(wal::WalCmd),
    ForceSend(force_send::ForceSendArgs),
    #[command(subcommand)]
    Agents(agents::AgentsCmd),
    #[command(subcommand)]
    Rules(rules::RulesCmd),
    #[command(subcommand)]
    Notifiers(notifiers::NotifiersCmd),
    #[command(subcommand)]
    Key(key::KeyCmd),
    Health(health::HealthArgs),
    Status(status::StatusArgs),
    TailLogs(tail_logs::TailLogsArgs),
    Version,
}

pub async fn run(opts: crate::Opts) -> Result<()> {
    let mode = opts.output_mode();
    match opts.cmd {
        Commands::Register(args) => register::execute(args, mode, opts.server, opts.config).await,
        Commands::Config(cmd) => config::execute(cmd, mode, opts.config).await,
        Commands::Wal(cmd) => wal::execute(cmd, mode, opts.config).await,
        Commands::ForceSend(args) => {
            force_send::execute(args, mode, opts.server, opts.config).await
        }
        Commands::Agents(cmd) => agents::execute(cmd, mode, opts.server, opts.config).await,
        Commands::Rules(cmd) => rules::execute(cmd, mode, opts.server, opts.config).await,
        Commands::Notifiers(cmd) => notifiers::execute(cmd, mode, opts.server).await,
        Commands::Key(cmd) => key::execute(cmd, mode, opts.config).await,
        Commands::Health(args) => health::execute(args, mode, opts.server, opts.config).await,
        Commands::Status(args) => status::execute(args, mode, opts.server, opts.config).await,
        Commands::TailLogs(args) => tail_logs::execute(args).await,
        Commands::Version => {
            version::execute(mode);
            Ok(())
        }
    }
}
