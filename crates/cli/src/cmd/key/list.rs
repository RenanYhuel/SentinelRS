use anyhow::Result;

use crate::output::{build_table, print_json, theme, OutputMode};

pub fn run(mode: OutputMode, config_path: Option<String>) -> Result<()> {
    let dir = super::rotate::key_store_dir(config_path.as_deref())?;

    let keys: Vec<String> = if dir.exists() {
        std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.strip_suffix(".enc").map(|s| s.to_string())
            })
            .collect()
    } else {
        Vec::new()
    };

    match mode {
        OutputMode::Json => {
            let items: Vec<_> = keys
                .iter()
                .map(|k| serde_json::json!({ "key_id": k }))
                .collect();
            print_json(&items)?;
        }
        OutputMode::Human => {
            if keys.is_empty() {
                theme::print_dim("  No keys stored.");
                return Ok(());
            }
            theme::print_header("Stored Keys");
            let mut table = build_table(&["#", "Key ID"]);
            for (i, k) in keys.iter().enumerate() {
                table.add_row(vec![(i + 1).to_string(), k.clone()]);
            }
            println!("{table}");
        }
    }

    Ok(())
}
