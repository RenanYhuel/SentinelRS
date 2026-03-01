use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use tonic::{Request, Response, Status};

use sentinel_common::proto::{RegisterRequest, RegisterResponse};
use sentinel_common::trace_id::generate_trace_id;
use crate::auth::generate_secret;
use crate::store::{AgentRecord, AgentStore};

pub async fn handle_register(
    store: &AgentStore,
    request: Request<RegisterRequest>,
) -> Result<Response<RegisterResponse>, Status> {
    let trace_id = generate_trace_id();
    let req = request.into_inner();
    let _span = tracing::info_span!("register", %trace_id, hw_id = %req.hw_id).entered();

    if req.hw_id.is_empty() {
        return Err(Status::invalid_argument("hw_id is required"));
    }

    if let Some(existing) = store.find_by_hw_id(&req.hw_id) {
        return Ok(Response::new(RegisterResponse {
            agent_id: existing.agent_id,
            secret: STANDARD.encode(&existing.secret),
        }));
    }

    let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
    let secret = generate_secret();
    let key_id = format!("key-{}", uuid::Uuid::new_v4());

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let record = AgentRecord {
        agent_id: agent_id.clone(),
        hw_id: req.hw_id,
        secret: secret.clone(),
        key_id,
        agent_version: req.agent_version,
        registered_at_ms: now_ms,
        deprecated_keys: Vec::new(),
    };

    store.insert(record);

    Ok(Response::new(RegisterResponse {
        agent_id,
        secret: STANDARD.encode(&secret),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_new_agent() {
        let store = AgentStore::new();
        let req = Request::new(RegisterRequest {
            hw_id: "hw-123".into(),
            agent_version: "0.1.0".into(),
        });
        let resp = handle_register(&store, req).await.unwrap();
        let inner = resp.into_inner();
        assert!(inner.agent_id.starts_with("agent-"));
        assert!(!inner.secret.is_empty());
        assert_eq!(store.count(), 1);
    }

    #[tokio::test]
    async fn register_same_hw_id_returns_existing() {
        let store = AgentStore::new();
        let req1 = Request::new(RegisterRequest {
            hw_id: "hw-123".into(),
            agent_version: "0.1.0".into(),
        });
        let resp1 = handle_register(&store, req1).await.unwrap().into_inner();

        let req2 = Request::new(RegisterRequest {
            hw_id: "hw-123".into(),
            agent_version: "0.2.0".into(),
        });
        let resp2 = handle_register(&store, req2).await.unwrap().into_inner();

        assert_eq!(resp1.agent_id, resp2.agent_id);
        assert_eq!(resp1.secret, resp2.secret);
        assert_eq!(store.count(), 1);
    }

    #[tokio::test]
    async fn register_empty_hw_id_fails() {
        let store = AgentStore::new();
        let req = Request::new(RegisterRequest {
            hw_id: "".into(),
            agent_version: "0.1.0".into(),
        });
        let err = handle_register(&store, req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }
}
