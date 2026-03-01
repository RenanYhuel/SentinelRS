use axum::http::StatusCode;
use axum::extract::State;
use super::state::AgentState;

pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub async fn ready(State(state): State<AgentState>) -> StatusCode {
    if state.is_ready() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn healthz_always_ok() {
        assert_eq!(healthz().await, StatusCode::OK);
    }

    #[tokio::test]
    async fn ready_reflects_state() {
        let state = AgentState::new();
        assert_eq!(ready(State(state.clone())).await, StatusCode::SERVICE_UNAVAILABLE);

        state.set_ready(true);
        assert_eq!(ready(State(state)).await, StatusCode::OK);
    }
}
