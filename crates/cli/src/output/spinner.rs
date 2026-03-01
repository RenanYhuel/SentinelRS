use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};
use colored::Colorize;

pub fn create(msg: &str) -> ProgressBar {
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::with_template("{spinner:.cyan.bold} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", " "]),
    );
    sp.set_message(msg.to_string());
    sp.enable_steady_tick(Duration::from_millis(80));
    sp
}

pub fn finish_ok(sp: &ProgressBar, msg: &str) {
    sp.set_style(ProgressStyle::with_template("{msg}").unwrap());
    sp.finish_with_message(format!("{} {}", "✓".green().bold(), msg));
}

pub fn finish_err(sp: &ProgressBar, msg: &str) {
    sp.set_style(ProgressStyle::with_template("{msg}").unwrap());
    sp.finish_with_message(format!("{} {}", "✗".red().bold(), msg));
}

pub fn finish_clear(sp: &ProgressBar) {
    sp.finish_and_clear();
}
