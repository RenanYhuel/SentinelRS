use sentinel_agent::plugin::{
    sign_blob, store_blob, store_manifest, load_blob, verify_blob,
    PluginManifest, PluginRuntime, ResourceLimits,
};
use tempfile::TempDir;

fn nginx_stub_manifest() -> PluginManifest {
    PluginManifest {
        name: "nginx_stub_status".into(),
        version: "1.0.0".into(),
        entry_fn: "collect".into(),
        capabilities: vec![],
        resource_limits: ResourceLimits::default(),
        metadata: Default::default(),
    }
}

const NGINX_STUB_WAT: &str = r#"
    (module
        (import "sentinel" "emit_metric_json" (func $emit (param i32 i32) (result i32)))
        (import "sentinel" "log" (func $log (param i32 i32)))
        (memory (export "memory") 1)
        (data (i32.const 0) "{\"name\":\"nginx.active_connections\",\"value\":42}")
        (data (i32.const 100) "{\"name\":\"nginx.requests_total\",\"value\":1337}")
        (data (i32.const 200) "nginx_stub_status: collected 2 metrics")
        (func (export "collect") (result i32)
            (call $emit (i32.const 0) (i32.const 49))
            drop
            (call $emit (i32.const 100) (i32.const 47))
            drop
            (call $log (i32.const 200) (i32.const 38))
            (i32.const 0)
        )
    )
"#;

#[test]
fn plugin_full_lifecycle_install_and_execute() {
    let dir = TempDir::new().unwrap();
    let key = b"test-signing-key-for-plugin";
    let wasm = NGINX_STUB_WAT.as_bytes();
    let signature = sign_blob(wasm, key);
    assert!(verify_blob(wasm, &signature, key));

    store_blob(dir.path(), "nginx_stub_status", wasm).unwrap();
    let manifest = nginx_stub_manifest();
    store_manifest(dir.path(), "nginx_stub_status", &manifest.to_yaml().unwrap()).unwrap();

    let loaded = load_blob(dir.path(), "nginx_stub_status").unwrap();
    let rt = PluginRuntime::load(&loaded, manifest).unwrap();
    let result = rt.execute().unwrap();

    assert_eq!(result.metrics_json.len(), 2);
    assert!(result.metrics_json[0].contains("nginx.active_connections"));
    assert!(result.metrics_json[1].contains("nginx.requests_total"));
    assert_eq!(result.logs.len(), 1);
    assert!(result.logs[0].contains("collected 2 metrics"));
}

#[test]
fn plugin_resource_limit_caps_metrics() {
    let mut manifest = nginx_stub_manifest();
    manifest.resource_limits.max_metrics = 1;
    let rt = PluginRuntime::load(NGINX_STUB_WAT.as_bytes(), manifest).unwrap();
    let result = rt.execute().unwrap();
    assert_eq!(result.metrics_json.len(), 1);
}

#[test]
fn plugin_tampered_blob_rejected() {
    let wasm = NGINX_STUB_WAT.as_bytes();
    let key = b"signing-key";
    let signature = sign_blob(wasm, key);
    let mut tampered = wasm.to_vec();
    tampered.push(0);
    assert!(!verify_blob(&tampered, &signature, key));
}
