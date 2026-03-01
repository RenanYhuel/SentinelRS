use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use sentinel_common::proto::agent_service_client::AgentServiceClient;
use sentinel_common::proto::{Batch, PushResponse};
use crate::security::HmacSigner;

pub struct GrpcClient {
    client: AgentServiceClient<Channel>,
    agent_id: String,
    signer: HmacSigner,
    key_id: String,
}

impl GrpcClient {
    pub async fn connect(
        endpoint: &str,
        agent_id: String,
        secret: &[u8],
        key_id: String,
    ) -> Result<Self, tonic::transport::Error> {
        let channel = Channel::from_shared(endpoint.to_string())
            .expect("valid endpoint")
            .connect()
            .await?;

        Ok(Self {
            client: AgentServiceClient::new(channel),
            agent_id,
            signer: HmacSigner::new(secret),
            key_id,
        })
    }

    pub async fn push_metrics(&mut self, batch: Batch) -> Result<PushResponse, tonic::Status> {
        let canonical =
            sentinel_common::canonicalize::canonical_bytes(&batch);
        let signature = self.signer.sign_base64(&canonical);

        let mut request = Request::new(batch);
        let metadata = request.metadata_mut();

        metadata.insert(
            "x-agent-id",
            MetadataValue::try_from(&self.agent_id).unwrap(),
        );
        metadata.insert(
            "x-signature",
            MetadataValue::try_from(&signature).unwrap(),
        );
        metadata.insert(
            "x-key-id",
            MetadataValue::try_from(&self.key_id).unwrap(),
        );

        self.client.push_metrics(request).await.map(|r| r.into_inner())
    }
}
