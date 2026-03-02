use std::path::PathBuf;

pub struct Args {
    pub config_path: PathBuf,
    pub legacy_mode: bool,
}

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
                println!("  -c, --config <PATH>  Configuration file path");
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

    match config_path {
        Some(p) => Args {
            config_path: p,
            legacy_mode,
        },
        None => {
            eprintln!("error: --config <path> is required");
            std::process::exit(1);
        }
    }
}
