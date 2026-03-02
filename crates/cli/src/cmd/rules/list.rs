use anyhow::Result;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching rules...")),
        OutputMode::Json => None,
    };

    let rules = api.get_json("/v1/rules").await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&rules)?,
        OutputMode::Human => {
            let empty = vec![];
            let arr = rules.as_array().unwrap_or(&empty);
            if arr.is_empty() {
                theme::print_dim("  No alert rules defined.");
                return Ok(());
            }
            theme::print_header("Alert Rules");
            let mut table = build_table(&["ID", "Name", "Metric", "Condition", "Threshold"]);
            for r in arr {
                table.add_row(vec![
                    r["id"].as_str().unwrap_or("-"),
                    r["name"].as_str().unwrap_or("-"),
                    r["metric_name"].as_str().unwrap_or("-"),
                    r["condition"].as_str().unwrap_or("-"),
                    &r["threshold"].to_string(),
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}
