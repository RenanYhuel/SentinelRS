use chrono::{DateTime, Utc};

pub fn format_relative(iso: &str) -> String {
    match iso.parse::<DateTime<Utc>>() {
        Ok(dt) => humanize(Utc::now() - dt),
        Err(_) => iso.to_string(),
    }
}

fn humanize(delta: chrono::TimeDelta) -> String {
    let secs = delta.num_seconds();
    if secs < 0 {
        return "just now".into();
    }
    if secs < 60 {
        return format!("{secs}s ago");
    }
    let mins = delta.num_minutes();
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = delta.num_hours();
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = delta.num_days();
    format!("{days}d ago")
}
