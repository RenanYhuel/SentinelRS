use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use super::health;
use super::metrics;
use super::state::AgentState;

pub fn router(state: AgentState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/ready", get(health::ready))
        .route("/metrics", get(metrics::metrics))
        .with_state(state)
}

pub async fn serve(listener: TcpListener, state: AgentState) -> std::io::Result<()> {
    let app = router(state);
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn send(app: Router, uri: &str) -> (StatusCode, String) {
        let req = Request::get(uri).body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn routes_respond() {
        let state = AgentState::new();
        state.set_ready(true);
        let app = router(state);

        let (status, _) = send(app.clone(), "/healthz").await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = send(app.clone(), "/ready").await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = send(app, "/metrics").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("sentinel_"));
    }
}
