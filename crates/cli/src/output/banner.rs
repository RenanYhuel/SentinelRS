use colored::Colorize;

const LOGO: &[&str] = &[
    r"   ███████╗███████╗███╗   ██╗████████╗██╗███╗   ██╗███████╗██╗     ",
    r"   ██╔════╝██╔════╝████╗  ██║╚══██╔══╝██║████╗  ██║██╔════╝██║     ",
    r"   ███████╗█████╗  ██╔██╗ ██║   ██║   ██║██╔██╗ ██║█████╗  ██║     ",
    r"   ╚════██║██╔══╝  ██║╚██╗██║   ██║   ██║██║╚██╗██║██╔══╝  ██║     ",
    r"   ███████║███████╗██║ ╚████║   ██║   ██║██║ ╚████║███████╗███████╗",
    r"   ╚══════╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝╚═╝  ╚═══╝╚══════╝╚══════╝",
    r"         ░░ Remote System Surveillance & Telemetry Engine ░░       ",
];

pub fn print_banner() {
    let colors = [
        colored::Color::Cyan,
        colored::Color::Cyan,
        colored::Color::BrightCyan,
        colored::Color::BrightWhite,
        colored::Color::Cyan,
        colored::Color::Cyan,
    ];

    for (line, &color) in LOGO.iter().zip(colors.iter()) {
        println!("{}", line.color(color).bold());
    }
}

pub fn print_version_block(version: &str) {
    print_banner();
    println!();
    println!("  {} {}", "Version".dimmed(), version.bright_cyan().bold());
    println!(
        "  {} {}",
        "Runtime".dimmed(),
        format!("Rust {}", rustc_version()).bright_white()
    );
    println!(
        "  {} {}",
        "  Arch ".dimmed(),
        std::env::consts::ARCH.bright_white()
    );
    println!(
        "  {} {}",
        "    OS ".dimmed(),
        std::env::consts::OS.bright_white()
    );
    println!();
}

fn rustc_version() -> &'static str {
    option_env!("RUSTC_VERSION")
        .or(option_env!("CARGO_PKG_RUST_VERSION"))
        .unwrap_or("stable")
}
