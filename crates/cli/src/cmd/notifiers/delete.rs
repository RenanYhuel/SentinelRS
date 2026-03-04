use anyhow::Result;
use clap::Args;

use crate::client;
use crate::output::{confirm, print_json, spinner, theme, OutputMode};

#[derive(Args)]
pub struct DeleteArgs {
    #[arg(help = "Notifier ID")]
    pub id: String,

    #[arg(long, help = "Skip confirmation")]
    pub yes: bool,
}

pub async fn run(args: DeleteArgs, mode: OutputMode, server: Option<String>) -> Result<()> {
    let api = client::build_client(server.as_deref())?;

    if mode == OutputMode::Human
        && !args.yes
        && !confirm::confirm_action(&format!("Delete notifier '{}'?", args.id))
    {
        theme::print_dim("  Cancelled.");
        return Ok(());
    }

    let sp = match mode {
        OutputMode::Human => Some(spinner::create("Deleting notifier...")),
        OutputMode::Json => None,
    };

    let status = api.delete_path(&format!("/v1/notifiers/{}", args.id)).await?;

    if let Some(sp) = sp {
        spinner::finish_ok(&sp, &format!("Notifier '{}' deleted", args.id));
    }

    if mode == OutputMode::Json {
        print_json(&serde_json::json!({
            "deleted": status.is_success() || status.as_u16() == 204,
            "id": args.id,
        }))?;
    }

    Ok(())
}
