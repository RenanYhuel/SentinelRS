use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::{Request, Status};

#[derive(Clone)]
pub struct AuthInterceptor {
    agent_id: String,
    key_id: Option<String>,
}

impl AuthInterceptor {
    pub fn new(agent_id: String, key_id: Option<String>) -> Self {
        Self { agent_id, key_id }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let meta = request.metadata_mut();
        meta.insert(
            "x-agent-id",
            MetadataValue::try_from(&self.agent_id)
                .map_err(|_| Status::internal("invalid agent id"))?,
        );
        if let Some(ref kid) = self.key_id {
            meta.insert(
                "x-key-id",
                MetadataValue::try_from(kid).map_err(|_| Status::internal("invalid key id"))?,
            );
        }
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interceptor_injects_headers_with_key_id() {
        let mut interceptor = AuthInterceptor::new("agent-1".into(), Some("key-abc".into()));
        let req = Request::new(());
        let result = interceptor.call(req).unwrap();
        let meta = result.metadata();
        assert_eq!(meta.get("x-agent-id").unwrap(), "agent-1");
        assert_eq!(meta.get("x-key-id").unwrap(), "key-abc");
    }

    #[test]
    fn interceptor_omits_key_id_when_none() {
        let mut interceptor = AuthInterceptor::new("agent-1".into(), None);
        let req = Request::new(());
        let result = interceptor.call(req).unwrap();
        let meta = result.metadata();
        assert_eq!(meta.get("x-agent-id").unwrap(), "agent-1");
        assert!(meta.get("x-key-id").is_none());
    }
}
