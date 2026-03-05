use anyhow::Result;
use clap::Args;

use crate::output::{print_json, theme, OutputMode};
use crate::store;

#[derive(Args)]
pub struct ShowArgs {
    #[arg(long, help = "Reveal secrets (tokens)")]
    pub reveal: bool,
}

pub fn run(args: ShowArgs, mode: OutputMode) -> Result<()> {
    let cfg = store::load()?;

    match mode {
        OutputMode::Json => {
            if args.reveal {
                print_json(&cfg)?;
            } else {
                print_json(&cfg.redacted())?;
            }
        }
        OutputMode::Human => {
            theme::print_header("CLI Configuration");
            theme::print_kv("Server URL", cfg.server_url());
            theme::print_kv("gRPC URL", &cfg.server.grpc_url);

            if cfg.auth.jwt_token.is_empty() {
                theme::print_kv("Token", "(none)");
            } else if args.reveal {
                theme::print_kv("Token", &cfg.auth.jwt_token);
            } else {
                theme::print_kv("Token", &cfg.masked_token());
            }

            if !cfg.auth.token_expires_at.is_empty() {
                theme::print_kv("Token Expires", &cfg.auth.token_expires_at);
            }

            theme::print_kv("Output", cfg.output());
            theme::print_kv("Color", &cfg.defaults.color.to_string());

            if !cfg.docker.compose_file.is_empty() {
                theme::print_kv("Compose File", &cfg.docker.compose_file);
            }
            if !cfg.docker.project_name.is_empty() {
                theme::print_kv("Docker Project", &cfg.docker.project_name);
            }

            println!();
            theme::print_dim(&format!("  File: {}", store::config_path().display()));
            println!();
        }
    }

    Ok(())
}
