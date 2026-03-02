use anyhow::Result;

use crate::client;
use crate::output::{print_json, spinner, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    if mode == OutputMode::Human {
        theme::print_header("Health Checks");
    }

    let sp_health = match mode {
        OutputMode::Human => Some(spinner::create("Checking /healthz ...")),
        OutputMode::Json => None,
    };

    let healthz = api.health_ok().await;

    if let Some(sp) = sp_health {
        if healthz {
            spinner::finish_ok(&sp, "Health check: OK");
        } else {
            spinner::finish_err(&sp, "Health check: FAILED");
        }
    }

    let sp_ready = match mode {
        OutputMode::Human => Some(spinner::create("Checking /ready ...")),
        OutputMode::Json => None,
    };

    let ready = api.get_text("/ready").await.is_ok();

    if let Some(sp) = sp_ready {
        if ready {
            spinner::finish_ok(&sp, "Ready check: OK");
        } else {
            spinner::finish_err(&sp, "Ready check: FAILED");
        }
    }

    match mode {
        OutputMode::Json => print_json(&serde_json::json!({
            "healthz": healthz,
            "ready": ready,
        }))?,
        OutputMode::Human => println!(),
    }

    Ok(())
}
