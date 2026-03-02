use anyhow::Result;

use crate::output::{banner, input, print_success, select, spinner, theme, OutputMode};
use crate::store::{self, CliConfig};

pub async fn execute(mode: OutputMode) -> Result<()> {
    if mode == OutputMode::Human {
        banner::print_banner();
        theme::print_header("Setup Wizard");
    }

    let server_url = input::text("Server URL", "http://localhost:8080")?;

    let sp = spinner::create("Testing connection...");
    let client = crate::client::ApiClient::new(&server_url);
    let reachable = client.health_ok().await;
    if reachable {
        spinner::finish_ok(&sp, "Server reachable");
    } else {
        spinner::finish_err(&sp, "Server unreachable (config saved anyway)");
    }

    let output_options = &["human", "json"];
    let output_idx = select::select_option("Default output format", output_options).unwrap_or(0);

    let cfg = CliConfig {
        server_url,
        output: output_options[output_idx].to_string(),
    };

    store::save(&cfg)?;

    if mode == OutputMode::Human {
        println!();
        print_success(&format!(
            "Config saved to {}",
            store::config_path().display()
        ));
        println!();
        theme::print_section("Saved Configuration");
        theme::print_kv("Server URL", &cfg.server_url);
        theme::print_kv("Output", &cfg.output);
        println!();
    }

    Ok(())
}
