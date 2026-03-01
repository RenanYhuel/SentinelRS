use std::path::PathBuf;

pub struct Args {
    pub config_path: PathBuf,
}

pub fn parse() -> Args {
    let mut args = std::env::args().skip(1);

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
                println!("  -V, --version        Print version");
                println!("  -h, --help           Print help");
                std::process::exit(0);
            }
            "--config" | "-c" => {
                let path = args.next().unwrap_or_else(|| {
                    eprintln!("error: --config requires a path argument");
                    std::process::exit(1);
                });
                return Args {
                    config_path: PathBuf::from(path),
                };
            }
            other => {
                eprintln!("error: unknown argument '{other}'");
                std::process::exit(1);
            }
        }
    }

    eprintln!("error: --config <path> is required");
    std::process::exit(1);
}
