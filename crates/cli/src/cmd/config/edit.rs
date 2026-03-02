use anyhow::Result;

use crate::output::{input, print_success, select, spinner, theme, OutputMode};
use crate::store::{self, CliConfig};

pub async fn run(mode: OutputMode) -> Result<()> {
    let current = store::load().unwrap_or_default();

    if mode == OutputMode::Human {
        theme::print_header("Edit Configuration");
    }

    let server_url = input::text("Server URL", &current.server_url)?;

    let sp = spinner::create("Testing connection...");
    let api = crate::client::ApiClient::new(&server_url);
    let ok = api.health_ok().await;
    if ok {
        spinner::finish_ok(&sp, "Server reachable");
    } else {
        spinner::finish_err(&sp, "Server unreachable (saving anyway)");
    }

    let output_options = &["human", "json"];
    let default_idx = if current.output == "json" { 1 } else { 0 };
    let idx = select::select_option("Default output format", output_options).unwrap_or(default_idx);

    let cfg = CliConfig {
        server_url,
        output: output_options[idx].to_string(),
    };

    store::save(&cfg)?;
    print_success("Configuration updated");

    Ok(())
}
