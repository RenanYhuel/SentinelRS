use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use crate::backpressure::{BatchSemaphore, CircuitBreaker};
use crate::identity::WorkerIdentity;
use crate::metrics::worker_metrics::WorkerMetrics;
use crate::registry::WorkerRegistry;

pub struct WorkerState {
    pub identity: Arc<WorkerIdentity>,
    pub metrics: Arc<WorkerMetrics>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub semaphore: Arc<BatchSemaphore>,
    pub in_flight: Arc<AtomicU64>,
    pub registry: Option<Arc<WorkerRegistry>>,
}
