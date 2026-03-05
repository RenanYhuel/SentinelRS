use anyhow::Result;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching notifiers...")),
        OutputMode::Json => None,
    };

    let notifiers = api.get_json("/v1/notifiers").await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&notifiers)?,
        OutputMode::Human => {
            let empty = vec![];
            let arr = notifiers.as_array().unwrap_or(&empty);
            if arr.is_empty() {
                theme::print_dim("  No notifiers configured.");
                return Ok(());
            }
            theme::print_header("Notifiers");
            let mut table = build_table(&["ID", "Name", "Type", "Enabled"]);
            for n in arr {
                table.add_row(vec![
                    n["id"].as_str().unwrap_or("-"),
                    n["name"].as_str().unwrap_or("-"),
                    n["ntype"].as_str().unwrap_or("-"),
                    if n["enabled"].as_bool().unwrap_or(false) {
                        "yes"
                    } else {
                        "no"
                    },
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}
