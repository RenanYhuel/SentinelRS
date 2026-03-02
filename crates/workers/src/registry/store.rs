use std::sync::Arc;
use std::time::Duration;

use async_nats::jetstream;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::config::RegistryConfig;
use crate::identity::WorkerIdentity;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerEntry {
    id: String,
    hostname: String,
    started_at: String,
    uptime_secs: u64,
}

pub struct WorkerRegistry {
    bucket: String,
    identity: Arc<WorkerIdentity>,
    peers: Arc<RwLock<Vec<String>>>,
    js: jetstream::Context,
}

impl WorkerRegistry {
    pub async fn create(
        js: jetstream::Context,
        config: &RegistryConfig,
        identity: Arc<WorkerIdentity>,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let kv_config = jetstream::kv::Config {
            bucket: config.bucket.clone(),
            max_value_size: 1024,
            history: 1,
            max_age: config.ttl,
            ..Default::default()
        };
        js.create_key_value(kv_config).await?;

        Ok(Arc::new(Self {
            bucket: config.bucket.clone(),
            identity,
            peers: Arc::new(RwLock::new(Vec::new())),
            js,
        }))
    }

    pub fn spawn_heartbeat(self: &Arc<Self>, interval: Duration, cancel: CancellationToken) {
        let registry = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        if let Err(e) = registry.deregister().await {
                            tracing::warn!(target: "registry", error = %e, "Failed to deregister on shutdown");
                        }
                        return;
                    }
                    _ = tokio::time::sleep(interval) => {
                        if let Err(e) = registry.heartbeat().await {
                            tracing::warn!(target: "registry", error = %e, "Registry heartbeat failed");
                        }
                        if let Err(e) = registry.refresh_peers().await {
                            tracing::warn!(target: "registry", error = %e, "Peer refresh failed");
                        }
                    }
                }
            }
        });
    }

    async fn heartbeat(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let kv = self.js.get_key_value(&self.bucket).await?;
        let entry = WorkerEntry {
            id: self.identity.id().to_string(),
            hostname: self.identity.hostname().to_string(),
            started_at: self.identity.started_at().to_string(),
            uptime_secs: self.identity.uptime_secs(),
        };
        let payload = serde_json::to_vec(&entry)?;
        kv.put(self.identity.id(), payload.into()).await?;
        Ok(())
    }

    async fn deregister(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let kv = self.js.get_key_value(&self.bucket).await?;
        kv.purge(self.identity.id()).await?;
        tracing::info!(target: "registry", worker = self.identity.id(), "Deregistered from cluster");
        Ok(())
    }

    async fn refresh_peers(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let kv = self.js.get_key_value(&self.bucket).await?;
        let mut keys = Vec::new();

        use futures::StreamExt;
        let mut key_stream = kv.keys().await?;
        while let Some(key) = key_stream.next().await {
            if let Ok(k) = key {
                keys.push(k);
            }
        }

        let mut guard = self.peers.write().await;
        *guard = keys;
        Ok(())
    }

    pub fn peers(&self) -> Vec<String> {
        self.peers.try_read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.try_read().map(|g| g.len()).unwrap_or(0)
    }
}
