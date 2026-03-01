use crate::output::{OutputMode, print_json};
use serde::Serialize;

#[derive(Serialize)]
struct VersionInfo {
    name: &'static str,
    version: &'static str,
}

pub fn execute(mode: OutputMode) {
    let info = VersionInfo {
        name: "SentinelRS CLI",
        version: env!("CARGO_PKG_VERSION"),
    };

    match mode {
        OutputMode::Json => { let _ = print_json(&info); }
        OutputMode::Human => println!("{} v{}", info.name, info.version),
    }
}
