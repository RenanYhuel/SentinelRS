use colored::Colorize;

pub fn render_horizontal(label: &str, value: f64, max: f64, width: usize) -> String {
    let ratio = if max > 0.0 { value / max } else { 0.0 };
    let filled = (ratio * width as f64).round() as usize;
    let bar: String = "█".repeat(filled);
    let empty: String = "░".repeat(width.saturating_sub(filled));
    let pct = (ratio * 100.0) as u64;

    format!(
        "  {:<20} {} {}%",
        label.dimmed(),
        format!("{bar}{empty}").cyan(),
        pct
    )
}

pub fn render_metric_bars(items: &[(&str, f64)], width: usize) {
    let max = items
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    for (label, value) in items {
        println!("{}", render_horizontal(label, *value, max, width));
    }
}
