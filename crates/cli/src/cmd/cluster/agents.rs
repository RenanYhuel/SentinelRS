use anyhow::Result;

use crate::client;
use crate::output::{build_table, print_json, spinner, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching connected agents...")),
        OutputMode::Json => None,
    };

    let ids = api.get_json("/v1/cluster/agents").await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&ids)?,
        OutputMode::Human => {
            let empty = vec![];
            let arr = ids.as_array().unwrap_or(&empty);
            if arr.is_empty() {
                theme::print_dim("  No agents currently connected.");
                return Ok(());
            }
            theme::print_header("Connected Agents");
            let mut table = build_table(&["#", "Agent ID"]);
            for (i, id) in arr.iter().enumerate() {
                table.add_row(vec![
                    (i + 1).to_string(),
                    id.as_str().unwrap_or("-").to_string(),
                ]);
            }
            println!("{table}");
        }
    }

    Ok(())
}
