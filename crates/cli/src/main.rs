use clap::Parser;

#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    cmd: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    Version,
}

fn main() {
    let opts = Opts::parse();
    match opts.cmd {
        Some(Commands::Version) => println!("SentinelRS CLI - version 0.1.0"),
        None => println!("SentinelRS CLI - use subcommands (e.g. version)"),
    }
}
