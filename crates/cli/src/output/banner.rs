use colored::Colorize;

const LOGO: &[&str] = &[
    r"   ███████╗███████╗███╗   ██╗████████╗██╗███╗   ██╗███████╗██╗     ",
    r"   ██╔════╝██╔════╝████╗  ██║╚══██╔══╝██║████╗  ██║██╔════╝██║     ",
    r"   ███████╗█████╗  ██╔██╗ ██║   ██║   ██║██╔██╗ ██║█████╗  ██║     ",
    r"   ╚════██║██╔══╝  ██║╚██╗██║   ██║   ██║██║╚██╗██║██╔══╝  ██║     ",
    r"   ███████║███████╗██║ ╚████║   ██║   ██║██║ ╚████║███████╗███████╗",
    r"   ╚══════╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝╚═╝  ╚═══╝╚══════╝╚══════╝",
];

const TAGLINE: &str = "Remote System Surveillance & Telemetry Engine";

pub fn print_banner() {
    let palette = [
        colored::Color::BrightCyan,
        colored::Color::Cyan,
        colored::Color::BrightCyan,
        colored::Color::BrightWhite,
        colored::Color::Cyan,
        colored::Color::BrightCyan,
    ];

    println!();
    for (i, line) in LOGO.iter().enumerate() {
        let color = palette[i % palette.len()];
        println!("{}", line.color(color).bold());
    }
    println!("         {}", format!("░░ {TAGLINE} ░░").dimmed());
    println!();
}

pub fn print_version_block(version: &str) {
    print_banner();
    let pairs = [
        ("Version", version.to_string()),
        ("Arch", std::env::consts::ARCH.to_string()),
        ("OS", std::env::consts::OS.to_string()),
    ];
    for (label, value) in &pairs {
        println!(
            "  {} {}",
            format!("{label:>8}").dimmed(),
            value.bright_cyan().bold()
        );
    }
    println!();
}
