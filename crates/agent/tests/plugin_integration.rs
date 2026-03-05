use sentinel_agent::config::PluginConfig;
use sentinel_agent::plugin::discovery::{list_installed, remove_plugin, scan_plugins_dir};
use sentinel_agent::plugin::PluginScheduler;
use sentinel_agent::plugin::{
    load_blob, sign_blob, store_blob, store_manifest, verify_blob, PluginManifest, PluginRuntime,
    ResourceLimits,
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
    store_manifest(
        dir.path(),
        "nginx_stub_status",
        &manifest.to_yaml().unwrap(),
    )
    .unwrap();

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

#[test]
fn discovery_scan_finds_installed_plugins() {
    let dir = TempDir::new().unwrap();
    let wasm = NGINX_STUB_WAT.as_bytes();
    let manifest = nginx_stub_manifest();

    store_blob(dir.path(), "nginx_stub_status", wasm).unwrap();
    store_manifest(
        dir.path(),
        "nginx_stub_status",
        &manifest.to_yaml().unwrap(),
    )
    .unwrap();

    let plugins = scan_plugins_dir(dir.path(), None);
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].name, "nginx_stub_status");
    assert_eq!(plugins[0].manifest.version, "1.0.0");
}

#[test]
fn discovery_with_signature_check() {
    let dir = TempDir::new().unwrap();
    let wasm = NGINX_STUB_WAT.as_bytes();
    let manifest = nginx_stub_manifest();
    let key = b"discovery-key";

    store_blob(dir.path(), "nginx_stub_status", wasm).unwrap();
    store_manifest(
        dir.path(),
        "nginx_stub_status",
        &manifest.to_yaml().unwrap(),
    )
    .unwrap();

    let sig = sign_blob(wasm, key);
    std::fs::write(dir.path().join("nginx_stub_status.sig"), &sig).unwrap();

    let plugins = scan_plugins_dir(dir.path(), Some(key));
    assert_eq!(plugins.len(), 1);
}

#[test]
fn discovery_rejects_unsigned_when_key_required() {
    let dir = TempDir::new().unwrap();
    let wasm = NGINX_STUB_WAT.as_bytes();
    let manifest = nginx_stub_manifest();

    store_blob(dir.path(), "nginx_stub_status", wasm).unwrap();
    store_manifest(
        dir.path(),
        "nginx_stub_status",
        &manifest.to_yaml().unwrap(),
    )
    .unwrap();

    let plugins = scan_plugins_dir(dir.path(), Some(b"require-key"));
    assert!(plugins.is_empty());
}

#[test]
fn list_installed_displays_all_plugins() {
    let dir = TempDir::new().unwrap();
    let manifest_a = PluginManifest {
        name: "alpha".into(),
        version: "1.0.0".into(),
        entry_fn: "collect".into(),
        capabilities: vec![],
        resource_limits: ResourceLimits::default(),
        metadata: Default::default(),
    };
    let manifest_b = PluginManifest {
        name: "beta".into(),
        version: "2.0.0".into(),
        entry_fn: "run".into(),
        capabilities: vec![],
        resource_limits: ResourceLimits::default(),
        metadata: Default::default(),
    };

    store_manifest(dir.path(), "alpha", &manifest_a.to_yaml().unwrap()).unwrap();
    store_manifest(dir.path(), "beta", &manifest_b.to_yaml().unwrap()).unwrap();

    let list = list_installed(dir.path());
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].0, "alpha");
    assert_eq!(list[1].0, "beta");
}

#[test]
fn remove_plugin_deletes_all_files() {
    let dir = TempDir::new().unwrap();
    let wasm = NGINX_STUB_WAT.as_bytes();
    let manifest = nginx_stub_manifest();

    store_blob(dir.path(), "nginx_stub_status", wasm).unwrap();
    store_manifest(
        dir.path(),
        "nginx_stub_status",
        &manifest.to_yaml().unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("nginx_stub_status.sig"), b"sig").unwrap();

    remove_plugin(dir.path(), "nginx_stub_status").unwrap();
    assert!(!dir.path().join("nginx_stub_status.wasm").exists());
    assert!(!dir.path().join("nginx_stub_status.manifest.yml").exists());
    assert!(!dir.path().join("nginx_stub_status.sig").exists());
}

#[test]
fn scheduler_discovers_and_counts_plugins() {
    let dir = TempDir::new().unwrap();
    let wasm = NGINX_STUB_WAT.as_bytes();
    let manifest = nginx_stub_manifest();

    store_blob(dir.path(), "nginx_stub_status", wasm).unwrap();
    store_manifest(
        dir.path(),
        "nginx_stub_status",
        &manifest.to_yaml().unwrap(),
    )
    .unwrap();

    let config = PluginConfig {
        enabled: true,
        dir: dir.path().to_str().unwrap().into(),
        interval_seconds: 60,
        signing_key: None,
    };

    let mut scheduler = PluginScheduler::new(config);
    scheduler.discover();
    assert_eq!(scheduler.loaded_count(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduler_spawns_and_collects_metrics() {
    let dir = TempDir::new().unwrap();
    let wasm = NGINX_STUB_WAT.as_bytes();
    let manifest = nginx_stub_manifest();

    store_blob(dir.path(), "nginx_stub_status", wasm).unwrap();
    store_manifest(
        dir.path(),
        "nginx_stub_status",
        &manifest.to_yaml().unwrap(),
    )
    .unwrap();

    let config = PluginConfig {
        enabled: true,
        dir: dir.path().to_str().unwrap().into(),
        interval_seconds: 1,
        signing_key: None,
    };

    let mut scheduler = PluginScheduler::new(config);
    scheduler.discover();

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    let handle = scheduler.spawn(tx);

    let metrics = tokio::time::timeout(std::time::Duration::from_secs(10), rx.recv())
        .await
        .expect("timed out waiting for plugin metrics")
        .expect("channel closed");

    assert!(!metrics.is_empty());
    assert!(metrics.iter().any(|m| m.name.contains("plugin.")));
    assert!(metrics.iter().any(|m| m.labels.contains_key("plugin")));

    handle.abort();
}

#[test]
fn multiple_plugins_discovered() {
    let dir = TempDir::new().unwrap();

    let wat_a = r#"
        (module
            (import "sentinel" "emit_metric_json" (func $emit (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "{\"name\":\"a\",\"value\":1}")
            (func (export "collect") (result i32)
                (call $emit (i32.const 0) (i32.const 22)) drop
                (i32.const 0)
            )
        )
    "#;
    let wat_b = r#"
        (module
            (import "sentinel" "emit_metric_json" (func $emit (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "{\"name\":\"b\",\"value\":2}")
            (func (export "collect") (result i32)
                (call $emit (i32.const 0) (i32.const 22)) drop
                (i32.const 0)
            )
        )
    "#;

    let manifest_a = PluginManifest {
        name: "plugin_a".into(),
        version: "1.0.0".into(),
        entry_fn: "collect".into(),
        capabilities: vec![],
        resource_limits: ResourceLimits::default(),
        metadata: Default::default(),
    };
    let manifest_b = PluginManifest {
        name: "plugin_b".into(),
        version: "1.0.0".into(),
        entry_fn: "collect".into(),
        capabilities: vec![],
        resource_limits: ResourceLimits::default(),
        metadata: Default::default(),
    };

    store_blob(dir.path(), "plugin_a", wat_a.as_bytes()).unwrap();
    store_manifest(dir.path(), "plugin_a", &manifest_a.to_yaml().unwrap()).unwrap();
    store_blob(dir.path(), "plugin_b", wat_b.as_bytes()).unwrap();
    store_manifest(dir.path(), "plugin_b", &manifest_b.to_yaml().unwrap()).unwrap();

    let config = PluginConfig {
        enabled: true,
        dir: dir.path().to_str().unwrap().into(),
        interval_seconds: 60,
        signing_key: None,
    };

    let mut scheduler = PluginScheduler::new(config);
    scheduler.discover();
    assert_eq!(scheduler.loaded_count(), 2);
}

#[test]
fn plugin_error_does_not_crash_discovery() {
    let dir = TempDir::new().unwrap();

    store_blob(dir.path(), "bad_plugin", b"not valid wasm").unwrap();
    store_manifest(
        dir.path(),
        "bad_plugin",
        "name: bad\nversion: '1.0'\nentry_fn: collect\n",
    )
    .unwrap();

    let good_wat = r#"
        (module
            (memory (export "memory") 1)
            (func (export "collect") (result i32) (i32.const 0))
        )
    "#;
    let good_manifest = PluginManifest {
        name: "good".into(),
        version: "1.0.0".into(),
        entry_fn: "collect".into(),
        capabilities: vec![],
        resource_limits: ResourceLimits::default(),
        metadata: Default::default(),
    };
    store_blob(dir.path(), "good_plugin", good_wat.as_bytes()).unwrap();
    store_manifest(dir.path(), "good_plugin", &good_manifest.to_yaml().unwrap()).unwrap();

    let config = PluginConfig {
        enabled: true,
        dir: dir.path().to_str().unwrap().into(),
        interval_seconds: 60,
        signing_key: None,
    };

    let mut scheduler = PluginScheduler::new(config);
    scheduler.discover();
    assert_eq!(scheduler.loaded_count(), 1);
}
