use chrono::{DateTime, Utc};

pub fn format_relative(iso: &str) -> String {
    let parsed = iso.parse::<DateTime<Utc>>();
    match parsed {
        Ok(dt) => relative(Utc::now() - dt),
        Err(_) => iso.to_string(),
    }
}

fn relative(delta: chrono::TimeDelta) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_timestamp() {
        let now = Utc::now() - chrono::Duration::seconds(30);
        let iso = now.to_rfc3339();
        let result = format_relative(&iso);
        assert!(result.ends_with("s ago"), "got: {result}");
    }

    #[test]
    fn minutes_ago() {
        let ts = Utc::now() - chrono::Duration::minutes(5);
        let result = format_relative(&ts.to_rfc3339());
        assert!(result.ends_with("m ago"), "got: {result}");
    }

    #[test]
    fn invalid_string() {
        let result = format_relative("not-a-date");
        assert_eq!(result, "not-a-date");
    }
}
