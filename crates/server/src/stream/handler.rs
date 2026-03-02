use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};

use sentinel_common::proto::sentinel_stream_server::SentinelStream;
use sentinel_common::proto::{
    agent_message::Payload as AgentPayload, server_message::Payload as ServerPayload, AgentMessage,
    HandshakeAck, HandshakeStatus, ServerMessage,
};

use crate::broker::BrokerPublisher;
use crate::store::{AgentStore, IdempotencyStore};

use super::authenticator::{authenticate_handshake, AuthOutcome};
use super::dispatcher;
use super::registry::SessionRegistry;
use super::session::Session;

const CHANNEL_BUFFER: usize = 128;
const DEFAULT_HEARTBEAT_INTERVAL_MS: i64 = 10_000;
const HANDSHAKE_TIMEOUT_SECS: u64 = 15;

pub struct StreamService<B: BrokerPublisher> {
    agents: AgentStore,
    idempotency: IdempotencyStore,
    broker: Arc<B>,
    registry: SessionRegistry,
    grace_period_ms: i64,
}

impl<B: BrokerPublisher> StreamService<B> {
    pub fn new(
        agents: AgentStore,
        idempotency: IdempotencyStore,
        broker: Arc<B>,
        registry: SessionRegistry,
        grace_period_ms: i64,
    ) -> Self {
        Self {
            agents,
            idempotency,
            broker,
            registry,
            grace_period_ms,
        }
    }
}

type OpenStreamStream =
    Pin<Box<dyn tokio_stream::Stream<Item = Result<ServerMessage, Status>> + Send>>;

#[tonic::async_trait]
impl<B: BrokerPublisher + 'static> SentinelStream for StreamService<B> {
    type OpenStreamStream = OpenStreamStream;

    async fn open_stream(
        &self,
        request: Request<Streaming<AgentMessage>>,
    ) -> Result<Response<Self::OpenStreamStream>, Status> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
        let tx = Arc::new(tx);

        let inbound = request.into_inner();
        let agents = self.agents.clone();
        let idempotency = self.idempotency.clone();
        let broker = self.broker.clone();
        let registry = self.registry.clone();
        let grace_period_ms = self.grace_period_ms;

        tokio::spawn(async move {
            if let Err(e) = run_stream(
                inbound,
                tx,
                agents,
                idempotency,
                broker,
                registry,
                grace_period_ms,
            )
            .await
            {
                tracing::debug!(error = %e, "stream ended");
            }
        });

        let output = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(output)))
    }
}

async fn run_stream<B: BrokerPublisher>(
    mut inbound: Streaming<AgentMessage>,
    tx: Arc<mpsc::Sender<Result<ServerMessage, Status>>>,
    agents: AgentStore,
    idempotency: IdempotencyStore,
    broker: Arc<B>,
    registry: SessionRegistry,
    grace_period_ms: i64,
) -> Result<(), StreamError> {
    let (agent_id, key_id) =
        wait_for_handshake(&mut inbound, &tx, &agents, &registry, grace_period_ms).await?;

    tracing::info!(agent_id = %agent_id, "stream authenticated");

    let result = message_loop(
        &agent_id,
        &key_id,
        &mut inbound,
        &tx,
        &agents,
        &idempotency,
        broker.as_ref(),
        &registry,
        grace_period_ms,
    )
    .await;

    registry.unregister(&agent_id);
    tracing::info!(agent_id = %agent_id, "stream disconnected");

    result
}

async fn wait_for_handshake(
    inbound: &mut Streaming<AgentMessage>,
    tx: &Arc<mpsc::Sender<Result<ServerMessage, Status>>>,
    agents: &AgentStore,
    registry: &SessionRegistry,
    grace_period_ms: i64,
) -> Result<(String, String), StreamError> {
    let first_msg = tokio::time::timeout(
        std::time::Duration::from_secs(HANDSHAKE_TIMEOUT_SECS),
        inbound.next(),
    )
    .await
    .map_err(|_| StreamError::HandshakeTimeout)?
    .ok_or(StreamError::StreamClosed)?
    .map_err(|e| StreamError::Transport(e.to_string()))?;

    let handshake = match first_msg.payload {
        Some(AgentPayload::Handshake(h)) => h,
        _ => {
            let _ = tx
                .send(Err(Status::unauthenticated(
                    "first message must be handshake",
                )))
                .await;
            return Err(StreamError::Protocol(
                "expected handshake as first message".into(),
            ));
        }
    };

    match authenticate_handshake(agents, &handshake, grace_period_ms) {
        AuthOutcome::Authenticated(auth) => {
            if registry.contains(&auth.agent_id) {
                registry.unregister(&auth.agent_id);
                tracing::warn!(agent_id = %auth.agent_id, "evicted stale session for reconnecting agent");
            }

            let session = Session::new(
                auth.agent_id.clone(),
                auth.agent_version.clone(),
                auth.capabilities.clone(),
                auth.key_id.clone(),
                DEFAULT_HEARTBEAT_INTERVAL_MS,
                tx.clone(),
            );
            registry.replace(session);

            let ack = ServerMessage {
                payload: Some(ServerPayload::HandshakeAck(HandshakeAck {
                    status: HandshakeStatus::HandshakeOk.into(),
                    message: "authenticated".into(),
                    server_time_ms: current_time_ms(),
                    heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
                })),
            };
            tx.send(Ok(ack))
                .await
                .map_err(|_| StreamError::StreamClosed)?;

            Ok((auth.agent_id, auth.key_id))
        }
        AuthOutcome::Rejected(ack) => {
            let msg = ServerMessage {
                payload: Some(ServerPayload::HandshakeAck(ack.clone())),
            };
            let _ = tx.send(Ok(msg)).await;
            Err(StreamError::AuthFailed(ack.message))
        }
    }
}

async fn message_loop<B: BrokerPublisher>(
    agent_id: &str,
    key_id: &str,
    inbound: &mut Streaming<AgentMessage>,
    tx: &Arc<mpsc::Sender<Result<ServerMessage, Status>>>,
    agents: &AgentStore,
    idempotency: &IdempotencyStore,
    broker: &B,
    registry: &SessionRegistry,
    grace_period_ms: i64,
) -> Result<(), StreamError> {
    while let Some(result) = inbound.next().await {
        let msg = result.map_err(|e| StreamError::Transport(e.to_string()))?;

        if let Some(response) = dispatcher::dispatch(
            agent_id,
            key_id,
            msg,
            agents,
            idempotency,
            broker,
            registry,
            grace_period_ms,
        )
        .await
        {
            tx.send(Ok(response))
                .await
                .map_err(|_| StreamError::StreamClosed)?;
        }
    }

    Ok(())
}

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[derive(Debug)]
enum StreamError {
    HandshakeTimeout,
    StreamClosed,
    Transport(String),
    Protocol(String),
    AuthFailed(String),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandshakeTimeout => write!(f, "handshake timeout"),
            Self::StreamClosed => write!(f, "stream closed"),
            Self::Transport(e) => write!(f, "transport: {e}"),
            Self::Protocol(e) => write!(f, "protocol: {e}"),
            Self::AuthFailed(e) => write!(f, "auth failed: {e}"),
        }
    }
}
