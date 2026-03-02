use crate::output::{banner, print_json, OutputMode};

pub fn run(mode: OutputMode) {
    let info = serde_json::json!({
        "name": "SentinelRS CLI",
        "version": env!("CARGO_PKG_VERSION"),
        "arch": std::env::consts::ARCH,
        "os": std::env::consts::OS,
    });

    match mode {
        OutputMode::Json => {
            let _ = print_json(&info);
        }
        OutputMode::Human => banner::print_version_block(env!("CARGO_PKG_VERSION")),
    }
}
