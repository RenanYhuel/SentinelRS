use tonic::{Request, Response, Status};

use crate::broker::BrokerPublisher;
use crate::store::{AgentStore, IdempotencyStore};
use sentinel_common::proto::agent_service_server::AgentService;
use sentinel_common::proto::{Batch, Heartbeat, PushResponse, RegisterRequest, RegisterResponse};

use super::heartbeat::handle_heartbeat;
use super::push_metrics::handle_push_metrics;
use super::register::handle_register;

pub struct AgentServiceImpl<B: BrokerPublisher> {
    agents: AgentStore,
    idempotency: IdempotencyStore,
    broker: B,
}

impl<B: BrokerPublisher> AgentServiceImpl<B> {
    pub fn new(agents: AgentStore, idempotency: IdempotencyStore, broker: B) -> Self {
        Self {
            agents,
            idempotency,
            broker,
        }
    }
}

#[tonic::async_trait]
impl<B: BrokerPublisher + 'static> AgentService for AgentServiceImpl<B> {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        handle_register(&self.agents, request).await
    }

    async fn push_metrics(
        &self,
        request: Request<Batch>,
    ) -> Result<Response<PushResponse>, Status> {
        handle_push_metrics(&self.agents, &self.idempotency, &self.broker, request).await
    }

    async fn send_heartbeat(
        &self,
        request: Request<Heartbeat>,
    ) -> Result<Response<PushResponse>, Status> {
        handle_heartbeat(&self.agents, request).await
    }
}
