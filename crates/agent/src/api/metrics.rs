use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use super::state::AgentState;

pub async fn metrics(State(state): State<AgentState>) -> impl IntoResponse {
    let body = format!(
        "# HELP sentinel_queue_length Number of unacked records in WAL\n\
         # TYPE sentinel_queue_length gauge\n\
         sentinel_queue_length {}\n\
         # HELP sentinel_wal_size_bytes Current WAL size on disk in bytes\n\
         # TYPE sentinel_wal_size_bytes gauge\n\
         sentinel_wal_size_bytes {}\n\
         # HELP sentinel_last_send_epoch Unix timestamp of last successful send\n\
         # TYPE sentinel_last_send_epoch gauge\n\
         sentinel_last_send_epoch {}\n\
         # HELP sentinel_batches_sent_total Total batches sent successfully\n\
         # TYPE sentinel_batches_sent_total counter\n\
         sentinel_batches_sent_total {}\n\
         # HELP sentinel_batches_failed_total Total batches that failed to send\n\
         # TYPE sentinel_batches_failed_total counter\n\
         sentinel_batches_failed_total {}\n",
        state.queue_length(),
        state.wal_size_bytes(),
        state.last_send_epoch(),
        state.batches_sent(),
        state.batches_failed(),
    );

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;

    #[tokio::test]
    async fn exposition_format() {
        let state = AgentState::new();
        state.set_queue_length(5);
        state.set_wal_size_bytes(2048);
        state.increment_batches_sent();

        let resp = metrics(State(state)).await.into_response();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(text.contains("sentinel_queue_length 5"));
        assert!(text.contains("sentinel_wal_size_bytes 2048"));
        assert!(text.contains("sentinel_batches_sent_total 1"));
        assert!(text.contains("# TYPE sentinel_queue_length gauge"));
        assert!(text.contains("# TYPE sentinel_batches_sent_total counter"));
    }
}
