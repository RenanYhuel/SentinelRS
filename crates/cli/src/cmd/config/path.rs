use anyhow::Result;

use crate::output::{print_json, OutputMode};
use crate::store;

pub fn run(mode: OutputMode) -> Result<()> {
    let p = store::config_path();
    match mode {
        OutputMode::Json => print_json(&serde_json::json!({ "path": p.display().to_string() }))?,
        OutputMode::Human => println!("{}", p.display()),
    }
    Ok(())
}
