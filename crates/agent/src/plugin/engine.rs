use super::error::PluginError;
use super::manifest::ResourceLimits;
use wasmtime::{Config, Engine, StoreLimits, StoreLimitsBuilder};

pub fn create_engine() -> Result<Engine, PluginError> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    Engine::new(&config).map_err(|e| PluginError::Compile(e.to_string()))
}

pub fn create_store_limits(limits: &ResourceLimits) -> StoreLimits {
    StoreLimitsBuilder::new()
        .memory_size(limits.max_memory_mb as usize * 1024 * 1024)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_creation_succeeds() {
        assert!(create_engine().is_ok());
    }

    #[test]
    fn store_limits_uses_manifest_values() {
        let limits = ResourceLimits {
            max_memory_mb: 32,
            timeout_ms: 1000,
            max_metrics: 500,
        };
        let _ = create_store_limits(&limits);
    }
}
