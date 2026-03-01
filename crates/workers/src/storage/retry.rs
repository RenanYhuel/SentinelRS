use super::writer::MetricWriter;

#[derive(Debug)]
pub enum WriteError {
    Sql(String),
    Serialization(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql(e) => write!(f, "sql: {e}"),
            Self::Serialization(e) => write!(f, "serialization: {e}"),
        }
    }
}

impl std::error::Error for WriteError {}

impl From<sqlx::Error> for WriteError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sql(e.to_string())
    }
}

impl From<serde_json::Error> for WriteError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

pub async fn write_with_retry(
    writer: &MetricWriter,
    rows: &[crate::transform::MetricRow],
    max_retries: u32,
) -> Result<u64, WriteError> {
    let mut attempt = 0u32;
    loop {
        match writer.insert_batch(rows).await {
            Ok(count) => return Ok(count),
            Err(e) if attempt < max_retries && is_transient(&e) => {
                attempt += 1;
                let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempt - 1));
                tracing::warn!(attempt, error = %e, "transient DB error, retrying");
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

fn is_transient(err: &WriteError) -> bool {
    match err {
        WriteError::Sql(msg) => {
            msg.contains("connection")
                || msg.contains("timeout")
                || msg.contains("too many clients")
                || msg.contains("deadlock")
        }
        WriteError::Serialization(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_detection() {
        assert!(is_transient(&WriteError::Sql("connection reset".into())));
        assert!(is_transient(&WriteError::Sql("timeout expired".into())));
        assert!(!is_transient(&WriteError::Sql("syntax error".into())));
        assert!(!is_transient(&WriteError::Serialization("bad json".into())));
    }

    #[test]
    fn error_display() {
        let e = WriteError::Sql("pg down".into());
        assert!(e.to_string().contains("sql"));
        let e = WriteError::Serialization("bad".into());
        assert!(e.to_string().contains("serialization"));
    }
}
