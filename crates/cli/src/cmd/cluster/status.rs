use anyhow::Result;

use crate::client;
use crate::output::{print_json, spinner, theme, OutputMode};

pub async fn run(mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Fetching cluster status...")),
        OutputMode::Json => None,
    };

    let stats = api.get_json("/v1/cluster/status").await?;

    if let Some(sp) = sp {
        spinner::finish_clear(&sp);
    }

    match mode {
        OutputMode::Json => print_json(&stats)?,
        OutputMode::Human => {
            theme::print_header("Cluster Status");
            for (k, v) in stats.as_object().into_iter().flatten() {
                theme::print_kv(k, &v.to_string());
            }
            println!();
        }
    }

    Ok(())
}
