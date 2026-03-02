use std::path::PathBuf;

pub struct Args {
    pub config_path: PathBuf,
    pub legacy_mode: bool,
}

const DEFAULT_CONFIG_PATH: &str = "/etc/sentinel/config.yml";

pub fn parse() -> Args {
    let mut args = std::env::args().skip(1);
    let mut config_path = None;
    let mut legacy_mode = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("sentinel_agent {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--help" | "-h" => {
                println!("Usage: sentinel_agent [OPTIONS]\n");
                println!("Options:");
                println!("  -c, --config <PATH>  Configuration file path (default: {DEFAULT_CONFIG_PATH})");
                println!("  --legacy-mode        Use V1 unary gRPC (no streaming)");
                println!("  -V, --version        Print version");
                println!("  -h, --help           Print help");
                std::process::exit(0);
            }
            "--config" | "-c" => {
                let path = args.next().unwrap_or_else(|| {
                    eprintln!("error: --config requires a path argument");
                    std::process::exit(1);
                });
                config_path = Some(PathBuf::from(path));
            }
            "--legacy-mode" | "--legacy" => {
                legacy_mode = true;
            }
            other => {
                eprintln!("error: unknown argument '{other}'");
                std::process::exit(1);
            }
        }
    }

    Args {
        config_path: config_path.unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH)),
        legacy_mode,
    }
}
