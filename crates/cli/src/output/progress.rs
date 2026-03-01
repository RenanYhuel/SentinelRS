use indicatif::{ProgressBar, ProgressStyle};

pub fn create(total: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "  {spinner:.cyan} [{bar:40.cyan/dim}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("━╸─"),
    );
    pb.set_message(msg.to_string());
    pb
}

pub fn finish(pb: &ProgressBar, msg: &str) {
    pb.set_style(
        ProgressStyle::with_template("  {msg}")
            .unwrap(),
    );
    pb.finish_with_message(format!("\x1b[32;1m✓\x1b[0m {msg}"));
}
