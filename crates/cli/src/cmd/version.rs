use crate::output::{banner, print_json, OutputMode};
use serde::Serialize;

#[derive(Serialize)]
struct VersionInfo {
    name: &'static str,
    version: &'static str,
    arch: &'static str,
    os: &'static str,
}

pub fn execute(mode: OutputMode) {
    let info = VersionInfo {
        name: "SentinelRS CLI",
        version: env!("CARGO_PKG_VERSION"),
        arch: std::env::consts::ARCH,
        os: std::env::consts::OS,
    };

    match mode {
        OutputMode::Json => {
            let _ = print_json(&info);
        }
        OutputMode::Human => banner::print_version_block(info.version),
    }
}
