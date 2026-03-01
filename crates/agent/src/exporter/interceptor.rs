use tonic::service::Interceptor;
use tonic::metadata::MetadataValue;
use tonic::{Request, Status};

#[derive(Clone)]
pub struct AuthInterceptor {
    agent_id: String,
    key_id: String,
}

impl AuthInterceptor {
    pub fn new(agent_id: String, key_id: String) -> Self {
        Self { agent_id, key_id }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let meta = request.metadata_mut();
        meta.insert(
            "x-agent-id",
            MetadataValue::try_from(&self.agent_id).map_err(|_| Status::internal("invalid agent id"))?,
        );
        meta.insert(
            "x-key-id",
            MetadataValue::try_from(&self.key_id).map_err(|_| Status::internal("invalid key id"))?,
        );
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interceptor_injects_headers() {
        let mut interceptor = AuthInterceptor::new("agent-1".into(), "key-abc".into());
        let req = Request::new(());
        let result = interceptor.call(req).unwrap();
        let meta = result.metadata();
        assert_eq!(meta.get("x-agent-id").unwrap(), "agent-1");
        assert_eq!(meta.get("x-key-id").unwrap(), "key-abc");
    }
}
