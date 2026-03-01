use wasmtime::StoreLimits;

pub struct HostState {
    pub collected_json: Vec<String>,
    pub logs: Vec<String>,
    pub limits: StoreLimits,
    pub max_metrics: u64,
}

impl HostState {
    pub fn new(limits: StoreLimits, max_metrics: u64) -> Self {
        Self {
            collected_json: Vec::new(),
            logs: Vec::new(),
            limits,
            max_metrics,
        }
    }
}
