use std::sync::Arc;
use wasmtime::{Engine, Linker, Module, Store};
use super::engine::{create_engine, create_store_limits};
use super::error::PluginError;
use super::host_fns::register_host_fns;
use super::host_state::HostState;
use super::manifest::PluginManifest;

pub struct PluginRuntime {
    engine: Arc<Engine>,
    module: Module,
    manifest: PluginManifest,
}

pub struct ExecutionResult {
    pub metrics_json: Vec<String>,
    pub logs: Vec<String>,
}

impl PluginRuntime {
    pub fn load(wasm_bytes: &[u8], manifest: PluginManifest) -> Result<Self, PluginError> {
        let engine = create_engine()?;
        let module =
            Module::new(&engine, wasm_bytes).map_err(|e| PluginError::Compile(e.to_string()))?;
        Ok(Self {
            engine: Arc::new(engine),
            module,
            manifest,
        })
    }

    pub fn execute(&self) -> Result<ExecutionResult, PluginError> {
        let limits = create_store_limits(&self.manifest.resource_limits);
        let state = HostState::new(limits, self.manifest.resource_limits.max_metrics);

        let mut store = Store::new(&self.engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(1);

        let mut linker = Linker::new(&self.engine);
        register_host_fns(&mut linker)?;

        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| PluginError::Instantiation(e.to_string()))?;

        let entry = instance
            .get_typed_func::<(), i32>(&mut store, &self.manifest.entry_fn)
            .map_err(|e| PluginError::Execution(e.to_string()))?;

        let engine = Arc::clone(&self.engine);
        let timeout_ms = self.manifest.resource_limits.timeout_ms;
        let guard = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
            engine.increment_epoch();
        });

        let result = entry.call(&mut store, ());
        drop(guard);

        match result {
            Ok(0) => {
                let st = store.into_data();
                Ok(ExecutionResult {
                    metrics_json: st.collected_json,
                    logs: st.logs,
                })
            }
            Ok(code) => Err(PluginError::Execution(format!("plugin returned code {code}"))),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("epoch") {
                    Err(PluginError::Timeout)
                } else if msg.contains("memory") {
                    Err(PluginError::MemoryLimit)
                } else {
                    Err(PluginError::Execution(msg))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::manifest::ResourceLimits;

    fn test_manifest(entry_fn: &str) -> PluginManifest {
        PluginManifest {
            name: "test_plugin".into(),
            version: "1.0.0".into(),
            entry_fn: entry_fn.into(),
            capabilities: vec![],
            resource_limits: ResourceLimits::default(),
            metadata: Default::default(),
        }
    }

    const WAT_EMIT_AND_LOG: &str = r#"
        (module
            (import "sentinel" "emit_metric_json" (func $emit (param i32 i32) (result i32)))
            (import "sentinel" "log" (func $log (param i32 i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "{\"name\":\"cpu\",\"value\":42}")
            (data (i32.const 50) "hello from plugin")
            (func (export "collect") (result i32)
                (call $emit (i32.const 0) (i32.const 27))
                drop
                (call $log (i32.const 50) (i32.const 17))
                (i32.const 0)
            )
        )
    "#;

    const WAT_RETURN_ERROR: &str = r#"
        (module
            (memory (export "memory") 1)
            (func (export "collect") (result i32)
                (i32.const 1)
            )
        )
    "#;

    const WAT_MULTI_EMIT: &str = r#"
        (module
            (import "sentinel" "emit_metric_json" (func $emit (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "{\"a\":1}")
            (data (i32.const 20) "{\"b\":2}")
            (func (export "collect") (result i32)
                (call $emit (i32.const 0) (i32.const 7))
                drop
                (call $emit (i32.const 20) (i32.const 7))
                drop
                (i32.const 0)
            )
        )
    "#;

    #[test]
    fn execute_plugin_emit_and_log() {
        let manifest = test_manifest("collect");
        let rt = PluginRuntime::load(WAT_EMIT_AND_LOG.as_bytes(), manifest).unwrap();
        let result = rt.execute().unwrap();
        assert_eq!(result.metrics_json.len(), 1);
        assert!(result.metrics_json[0].contains("cpu"));
        assert_eq!(result.logs.len(), 1);
        assert_eq!(result.logs[0], "hello from plugin");
    }

    #[test]
    fn plugin_nonzero_return_is_error() {
        let manifest = test_manifest("collect");
        let rt = PluginRuntime::load(WAT_RETURN_ERROR.as_bytes(), manifest).unwrap();
        assert!(matches!(rt.execute(), Err(PluginError::Execution(_))));
    }

    #[test]
    fn plugin_multi_emit() {
        let manifest = test_manifest("collect");
        let rt = PluginRuntime::load(WAT_MULTI_EMIT.as_bytes(), manifest).unwrap();
        let result = rt.execute().unwrap();
        assert_eq!(result.metrics_json.len(), 2);
    }

    #[test]
    fn plugin_max_metrics_enforced() {
        let mut manifest = test_manifest("collect");
        manifest.resource_limits.max_metrics = 1;
        let rt = PluginRuntime::load(WAT_MULTI_EMIT.as_bytes(), manifest).unwrap();
        let result = rt.execute().unwrap();
        assert_eq!(result.metrics_json.len(), 1);
    }

    #[test]
    fn invalid_wasm_returns_compile_error() {
        let manifest = test_manifest("collect");
        let result = PluginRuntime::load(b"not wasm", manifest);
        assert!(matches!(result, Err(PluginError::Compile(_))));
    }

    #[test]
    fn missing_entry_fn_returns_error() {
        let manifest = test_manifest("nonexistent");
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "collect") (result i32) (i32.const 0))
            )
        "#;
        let rt = PluginRuntime::load(wat.as_bytes(), manifest).unwrap();
        assert!(matches!(rt.execute(), Err(PluginError::Execution(_))));
    }
}
