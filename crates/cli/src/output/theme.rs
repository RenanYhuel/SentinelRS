use colored::Colorize;

pub fn print_header(title: &str) {
    let width = title.len() + 6;
    let border = "─".repeat(width);
    println!();
    println!("  ╭{}╮", border.cyan());
    println!("  │   {}   │", title.bright_cyan().bold());
    println!("  ╰{}╯", border.cyan());
    println!();
}

pub fn print_section(title: &str) {
    println!();
    println!("  {} {}", "●".bright_cyan(), title.bold());
    println!("  {}", "─".repeat(40).dimmed());
}

pub fn print_kv(label: &str, value: &str) {
    println!(
        "    {} {}",
        format!("{:<16}", label).dimmed(),
        value.bright_white()
    );
}

pub fn print_kv_colored(label: &str, value: &str, ok: bool) {
    let styled = if ok {
        value.green().to_string()
    } else {
        value.red().to_string()
    };
    println!("    {} {}", format!("{:<16}", label).dimmed(), styled);
}

#[allow(dead_code)]
pub fn print_warning(msg: &str) {
    println!("{} {}", "⚠".yellow().bold(), msg.yellow());
}

pub fn print_dim(msg: &str) {
    println!("  {}", msg.dimmed());
}

#[allow(dead_code)]
pub fn divider() {
    println!("  {}", "─".repeat(50).dimmed());
}
