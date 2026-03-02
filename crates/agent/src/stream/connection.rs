use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::transport::Channel;

use sentinel_common::proto::sentinel_stream_client::SentinelStreamClient;
use sentinel_common::proto::AgentMessage;

use crate::buffer::Wal;
use crate::security::HmacSigner;

use super::handshake::{build_handshake_message, validate_handshake_ack, HandshakeParams};
use super::receiver;
use super::reconnect::ReconnectPolicy;
use super::sender::StreamSender;
use super::wal_drain;

const OUTBOUND_BUFFER: usize = 256;
const HANDSHAKE_TIMEOUT_SECS: u64 = 15;

pub struct StreamClient {
    endpoint: String,
    agent_id: String,
    agent_version: String,
    key_id: String,
    signer: HmacSigner,
    wal: Arc<Mutex<Wal>>,
    reconnect: ReconnectPolicy,
}

impl StreamClient {
    pub fn new(
        endpoint: String,
        agent_id: String,
        agent_version: String,
        key_id: String,
        secret: &[u8],
        wal: Arc<Mutex<Wal>>,
    ) -> Self {
        Self {
            endpoint,
            agent_id,
            agent_version,
            key_id,
            signer: HmacSigner::new(secret),
            wal,
            reconnect: ReconnectPolicy::default(),
        }
    }

    pub async fn run(&self, _heartbeat_sender: Option<StreamSender>) -> ! {
        let mut attempt: u32 = 0;

        loop {
            match self.connect_and_run().await {
                Ok(()) => {
                    tracing::info!(target: "conn", "Stream closed gracefully");
                    attempt = 0;
                }
                Err(e) => {
                    tracing::warn!(target: "conn", error = %e, attempt, "Stream connection failed");
                }
            }

            let delay = self.reconnect.delay_for_attempt(attempt);
            tracing::info!(target: "conn", delay_ms = delay.as_millis() as u64, "Reconnecting");
            tokio::time::sleep(delay).await;
            attempt = attempt.saturating_add(1);
        }
    }

    async fn connect_and_run(&self) -> Result<(), ConnectionError> {
        let channel = Channel::from_shared(self.endpoint.clone())
            .map_err(|e| ConnectionError::Config(e.to_string()))?
            .connect()
            .await
            .map_err(|e| ConnectionError::Transport(e.to_string()))?;

        let mut client = SentinelStreamClient::new(channel);

        let (outbound_tx, outbound_rx) = mpsc::channel::<AgentMessage>(OUTBOUND_BUFFER);
        let outbound_stream = ReceiverStream::new(outbound_rx);

        let params = HandshakeParams {
            agent_id: self.agent_id.clone(),
            agent_version: self.agent_version.clone(),
            capabilities: vec!["metrics".into(), "heartbeat".into()],
            key_id: self.key_id.clone(),
        };

        let handshake_msg = build_handshake_message(&params, &self.signer);
        outbound_tx
            .send(handshake_msg)
            .await
            .map_err(|_| ConnectionError::ChannelClosed)?;

        let response = client
            .open_stream(outbound_stream)
            .await
            .map_err(|e| ConnectionError::Rpc(e.to_string()))?;

        let mut inbound = response.into_inner();

        let ack_msg =
            tokio::time::timeout(Duration::from_secs(HANDSHAKE_TIMEOUT_SECS), inbound.next())
                .await
                .map_err(|_| ConnectionError::HandshakeTimeout)?
                .ok_or(ConnectionError::StreamClosed)?
                .map_err(|e| ConnectionError::Transport(e.to_string()))?;

        let heartbeat_interval_ms = validate_handshake_ack(&ack_msg)
            .map_err(|e| ConnectionError::Handshake(e.to_string()))?;

        tracing::info!(
            target: "conn",
            agent_id = %self.agent_id,
            heartbeat_interval_ms,
            "Stream authenticated"
        );

        let sender = StreamSender::new(
            outbound_tx,
            self.agent_id.clone(),
            HmacSigner::new(&self.signer_secret()),
        );

        let heartbeat_interval = Duration::from_millis(heartbeat_interval_ms.max(1000) as u64);
        let heartbeat_sender = sender.clone();
        let heartbeat_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(heartbeat_interval).await;
                if heartbeat_sender.send_heartbeat().await.is_err() {
                    break;
                }
            }
        });

        let drain_sender = sender.clone();
        let drain_wal = self.wal.clone();
        let drain_handle = tokio::spawn(async move {
            wal_drain::drain_loop(drain_sender, drain_wal).await;
        });

        let recv_result = receiver::receive_loop(inbound, self.wal.clone()).await;

        heartbeat_handle.abort();
        drain_handle.abort();

        match recv_result {
            Ok(()) => Ok(()),
            Err(receiver::RecvError::StreamEnded) => Ok(()),
            Err(e) => Err(ConnectionError::Receive(e.to_string())),
        }
    }

    fn signer_secret(&self) -> Vec<u8> {
        self.signer.secret_bytes()
    }

    pub fn create_sender(&self) -> (StreamSender, mpsc::Receiver<AgentMessage>) {
        let (tx, rx) = mpsc::channel(OUTBOUND_BUFFER);
        let sender = StreamSender::new(
            tx,
            self.agent_id.clone(),
            HmacSigner::new(&self.signer_secret()),
        );
        (sender, rx)
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    Config(String),
    Transport(String),
    ChannelClosed,
    Rpc(String),
    HandshakeTimeout,
    StreamClosed,
    Handshake(String),
    Receive(String),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(e) => write!(f, "config: {e}"),
            Self::Transport(e) => write!(f, "transport: {e}"),
            Self::ChannelClosed => write!(f, "outbound channel closed"),
            Self::Rpc(e) => write!(f, "rpc: {e}"),
            Self::HandshakeTimeout => write!(f, "handshake timeout"),
            Self::StreamClosed => write!(f, "stream closed by server"),
            Self::Handshake(e) => write!(f, "handshake: {e}"),
            Self::Receive(e) => write!(f, "receive: {e}"),
        }
    }
}
