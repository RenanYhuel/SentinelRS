use tonic::{Request, Response, Status};

use sentinel_common::proto::push_response::Status as PushStatus;
use sentinel_common::proto::{Heartbeat, PushResponse};
use crate::store::AgentStore;

pub async fn handle_heartbeat(
    agents: &AgentStore,
    request: Request<Heartbeat>,
) -> Result<Response<PushResponse>, Status> {
    let hb = request.into_inner();

    if hb.agent_id.is_empty() {
        return Err(Status::invalid_argument("agent_id is required"));
    }

    if agents.get(&hb.agent_id).is_none() {
        return Err(Status::not_found("agent not registered"));
    }

    tracing::debug!(agent_id = %hb.agent_id, ts = hb.ts_ms, "heartbeat received");

    Ok(Response::new(PushResponse {
        status: PushStatus::Ok.into(),
        message: "ok".into(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::AgentRecord;

    fn make_store() -> AgentStore {
        let store = AgentStore::new();
        store.insert(AgentRecord {
            agent_id: "agent-1".into(),
            hw_id: "hw-1".into(),
            secret: vec![],
            key_id: "key-1".into(),
            agent_version: "0.1.0".into(),
            registered_at_ms: 1000,
        });
        store
    }

    #[tokio::test]
    async fn valid_heartbeat() {
        let store = make_store();
        let req = Request::new(Heartbeat {
            agent_id: "agent-1".into(),
            ts_ms: 2000,
            info: Default::default(),
        });
        let resp = handle_heartbeat(&store, req).await.unwrap();
        assert_eq!(resp.into_inner().status, PushStatus::Ok as i32);
    }

    #[tokio::test]
    async fn unknown_agent_heartbeat() {
        let store = make_store();
        let req = Request::new(Heartbeat {
            agent_id: "unknown".into(),
            ts_ms: 2000,
            info: Default::default(),
        });
        let err = handle_heartbeat(&store, req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn empty_agent_id_heartbeat() {
        let store = make_store();
        let req = Request::new(Heartbeat {
            agent_id: "".into(),
            ts_ms: 0,
            info: Default::default(),
        });
        let err = handle_heartbeat(&store, req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }
}
